use std::{cmp::Ordering, collections::BTreeMap, fs, path::Path};

use brk_cohort::{AGE_RANGE_NAMES, CohortContext};
use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Cents, CentsCompact, Date, Day1, Sats, StoredF64, UrpdRaw, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecValue, WritableVec};

use super::vecs::{Levels, MODE_COUNT, ModeVecs, Percentiles, Vecs};
use crate::{
    distribution,
    frameworks::{
        coinflow::{self, AGE_COHORT_COUNT, AgeBand, HORIZON_COUNT, HORIZON_DAYS, age_bounds_days},
        cointime,
    },
    indexes,
    internal::db_utils::validate_any_computed_version_or_reset,
};

const MIN_CALIBRATION_DAYS: usize = 365;
const WRITE_INTERVAL_DAYS: usize = 100;
const PERCENTILE_COUNT: usize = 5;
const LEVEL_COUNT: usize = 9;
const PERCENTILES: [f64; PERCENTILE_COUNT] = [0.95, 0.98, 0.99, 0.995, 0.999];
const LEVEL_PERCENTILES: [f64; LEVEL_COUNT] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

const RAW_MODE: usize = 0;
const COINTIME_MODE: usize = 1;
const COINFLOW_MODE: usize = 2;
const COINFLOW_HORIZON_START: usize = 3;

type Thresholds = [Option<[f64; PERCENTILE_COUNT]>; MODE_COUNT];
type ModeWeights = [Option<[f64; AGE_COHORT_COUNT]>; MODE_COUNT];

struct DayResult {
    loss_threshold: [[StoredF64; PERCENTILE_COUNT]; MODE_COUNT],
    floor: [[Cents; PERCENTILE_COUNT]; MODE_COUNT],
    level: [[Cents; LEVEL_COUNT]; MODE_COUNT],
}

impl DayResult {
    fn from_thresholds(thresholds: &Thresholds) -> Self {
        Self {
            loss_threshold: thresholds.map(|thresholds| {
                thresholds
                    .map(|values| values.map(StoredF64::from))
                    .unwrap_or([StoredF64::NAN; PERCENTILE_COUNT])
            }),
            floor: [[Cents::NAN; PERCENTILE_COUNT]; MODE_COUNT],
            level: [[Cents::NAN; LEVEL_COUNT]; MODE_COUNT],
        }
    }
}

struct Calibration {
    histories: [Vec<f64>; MODE_COUNT],
}

impl Calibration {
    fn from_sources<T, U>(
        raw: &impl ReadableVec<Day1, Option<T>>,
        weighted: &[&impl ReadableVec<Day1, Option<U>>],
        end: usize,
    ) -> Self
    where
        T: VecValue,
        U: VecValue,
        f64: From<T> + From<U>,
    {
        let mut histories = std::array::from_fn(|_| Vec::new());
        histories[RAW_MODE] = collect_loss_history(raw, end);
        for (history, source) in histories[1..].iter_mut().zip(weighted) {
            *history = collect_loss_history(*source, end);
        }
        Self { histories }
    }

    fn thresholds(&self, current: &[Option<f64>; MODE_COUNT]) -> Thresholds {
        std::array::from_fn(|mode| {
            (current[mode].is_some() && self.histories[mode].len() >= MIN_CALIBRATION_DAYS).then(
                || {
                    PERCENTILES.map(|percentile| {
                        quantile(&self.histories[mode], percentile).expect("non-empty history")
                    })
                },
            )
        })
    }

    fn observe(&mut self, shares: [Option<f64>; MODE_COUNT]) {
        for (history, share) in self.histories.iter_mut().zip(shares) {
            if let Some(share) = share {
                insert_sorted(history, share.clamp(0.0, 1.0));
            }
        }
    }
}

impl<T> Percentiles<T> {
    fn as_mut_array(&mut self) -> [&mut T; PERCENTILE_COUNT] {
        [
            &mut self.pct95,
            &mut self.pct98,
            &mut self.pct99,
            &mut self.pct99_5,
            &mut self.pct99_9,
        ]
    }
}

impl<T> Levels<T> {
    fn as_mut_array(&mut self) -> [&mut T; LEVEL_COUNT] {
        [
            &mut self.pct10,
            &mut self.pct20,
            &mut self.pct30,
            &mut self.pct40,
            &mut self.pct50,
            &mut self.pct60,
            &mut self.pct70,
            &mut self.pct80,
            &mut self.pct90,
        ]
    }
}

impl ModeVecs {
    fn stored_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs: Vec<&mut dyn AnyStoredVec> =
            Vec::with_capacity(2 * PERCENTILE_COUNT + LEVEL_COUNT);
        vecs.extend(
            self.loss_threshold
                .as_mut_array()
                .into_iter()
                .map(|vec| &mut vec.day1 as &mut dyn AnyStoredVec),
        );
        vecs.extend(
            self.floor
                .as_mut_array()
                .into_iter()
                .map(|vec| &mut vec.cents.day1 as &mut dyn AnyStoredVec),
        );
        vecs.extend(
            self.level
                .as_mut_array()
                .into_iter()
                .map(|vec| &mut vec.cents.day1 as &mut dyn AnyStoredVec),
        );
        vecs
    }

    fn push(
        &mut self,
        loss_threshold: [StoredF64; PERCENTILE_COUNT],
        floor: [Cents; PERCENTILE_COUNT],
        level: [Cents; LEVEL_COUNT],
    ) {
        for (vec, value) in self
            .loss_threshold
            .as_mut_array()
            .into_iter()
            .zip(loss_threshold)
        {
            vec.day1.push(value);
        }
        for (vec, value) in self.floor.as_mut_array().into_iter().zip(floor) {
            vec.cents.day1.push(value);
        }
        for (vec, value) in self.level.as_mut_array().into_iter().zip(level) {
            vec.cents.day1.push(value);
        }
    }
}

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        distribution: &distribution::Vecs,
        cointime: &cointime::Vecs,
        coinflow: &coinflow::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let cointime_liveliness: Vec<_> = cointime
            .age_range
            .iter()
            .map(|cohort| &cohort.liveliness.day1)
            .collect();
        let age_supplies: Vec<_> = distribution
            .utxo_cohorts
            .age_range
            .iter()
            .map(|cohort| &cohort.metrics.supply.total.sats.day1)
            .collect();
        let coinflow_mobility: Vec<_> = coinflow
            .age_range
            .iter()
            .map(|cohort| &cohort.mobility.day1.0)
            .collect();
        let coinflow_spending_rate: Vec<_> = coinflow
            .age_range
            .iter()
            .map(|cohort| &cohort.spending_rate.day1)
            .collect();
        let raw_loss_share = &distribution
            .utxo_cohorts
            .all
            .metrics
            .relative
            .supply_in_loss_share
            .ppm
            .day1;
        let weighted_loss_shares: Vec<_> = [
            &cointime.supply.active_supply_in_loss_share.day1,
            &coinflow.supply_in_loss_share.day1,
        ]
        .into_iter()
        .chain(
            coinflow
                .horizon
                .iter()
                .map(|horizon| &horizon.supply_in_loss_share.day1),
        )
        .collect();
        debug_assert_eq!(weighted_loss_shares.len(), MODE_COUNT - 1);

        let source_version: Version = std::iter::once(indexes.day1.date.version())
            .chain(std::iter::once(distribution.supply_state.version()))
            .chain(std::iter::once(raw_loss_share.version()))
            .chain(weighted_loss_shares.iter().map(|vec| vec.version()))
            .chain(age_supplies.iter().map(|vec| vec.version()))
            .chain(cointime_liveliness.iter().map(|vec| vec.version()))
            .chain(coinflow_mobility.iter().map(|vec| vec.version()))
            .chain(coinflow_spending_rate.iter().map(|vec| vec.version()))
            .sum();

        for vec in self.stored_vecs_mut() {
            validate_any_computed_version_or_reset(vec, source_version)?;
        }

        let source_end = std::iter::once(indexes.day1.date.len())
            .chain(std::iter::once(raw_loss_share.len()))
            .chain(weighted_loss_shares.iter().map(|vec| vec.len()))
            .chain(age_supplies.iter().map(|vec| vec.len()))
            .chain(cointime_liveliness.iter().map(|vec| vec.len()))
            .chain(coinflow_mobility.iter().map(|vec| vec.len()))
            .chain(coinflow_spending_rate.iter().map(|vec| vec.len()))
            .min()
            .unwrap_or_default();
        let recompute_from = recompute_day(indexer, indexes)
            .map(usize::from)
            .unwrap_or_default();
        let start = self.minimum_len().min(recompute_from).min(source_end);

        for vec in self.stored_vecs_mut() {
            vec.any_truncate_if_needed_at(start)?;
        }

        let mut calibration =
            Calibration::from_sources(raw_loss_share, &weighted_loss_shares, start);
        let bounds = age_bounds_days();

        for day_index in start..source_end {
            let day = Day1::from(day_index);
            let loss_shares = collect_loss_shares(raw_loss_share, &weighted_loss_shares, day);
            let thresholds = calibration.thresholds(&loss_shares);
            let mut result = DayResult::from_thresholds(&thresholds);

            if thresholds.iter().any(Option::is_some)
                && let Some(date) = indexes.day1.date.collect_one(day)
            {
                let weights = mode_weights(
                    day,
                    &age_supplies,
                    &cointime_liveliness,
                    &coinflow_mobility,
                    &coinflow_spending_rate,
                    &bounds,
                );
                if let Some(weighted) =
                    read_weighted_urpd(&distribution.states_path, date, &weights)?
                {
                    evaluate_day(&weighted, &thresholds, &mut result);
                }
            }
            calibration.observe(loss_shares);

            for (mode, output) in self.modes.as_mut_array().into_iter().enumerate() {
                output.push(
                    result.loss_threshold[mode],
                    result.floor[mode],
                    result.level[mode],
                );
            }

            if (day_index + 1).is_multiple_of(WRITE_INTERVAL_DAYS) || day_index + 1 == source_end {
                let _lock = exit.lock();
                for vec in self.stored_vecs_mut() {
                    vec.write()?;
                }
            }
        }

        Ok(())
    }

    fn stored_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = Vec::with_capacity(MODE_COUNT * (2 * PERCENTILE_COUNT + LEVEL_COUNT));
        for mode in self.modes.as_mut_array() {
            vecs.extend(mode.stored_vecs_mut());
        }
        vecs
    }

    fn minimum_len(&mut self) -> usize {
        self.stored_vecs_mut()
            .into_iter()
            .map(|vec| vec.len())
            .min()
            .unwrap_or_default()
    }
}

fn collect_loss_history<T>(source: &impl ReadableVec<Day1, Option<T>>, end: usize) -> Vec<f64>
where
    T: VecValue,
    f64: From<T>,
{
    let mut history: Vec<_> = source
        .collect_range_at(0, end)
        .into_iter()
        .flatten()
        .map(f64::from)
        .filter(|value| value.is_finite())
        .collect();
    history.sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    history
}

fn collect_loss_share<T>(source: &impl ReadableVec<Day1, Option<T>>, day: Day1) -> Option<f64>
where
    T: VecValue,
    f64: From<T>,
{
    source
        .collect_one(day)
        .flatten()
        .map(f64::from)
        .filter(|value| value.is_finite())
}

fn collect_loss_shares<T, U>(
    raw: &impl ReadableVec<Day1, Option<T>>,
    weighted: &[&impl ReadableVec<Day1, Option<U>>],
    day: Day1,
) -> [Option<f64>; MODE_COUNT]
where
    T: VecValue,
    U: VecValue,
    f64: From<T> + From<U>,
{
    let mut shares = [None; MODE_COUNT];
    shares[RAW_MODE] = collect_loss_share(raw, day);
    for (share, source) in shares[1..].iter_mut().zip(weighted) {
        *share = collect_loss_share(*source, day);
    }
    shares
}

fn recompute_day(indexer: &Indexer, indexes: &indexes::Vecs) -> Option<Day1> {
    let starting_height = indexer.safe_lengths().height;
    indexes
        .height
        .day1
        .collect_one(starting_height)
        .or_else(|| {
            starting_height
                .decremented()
                .and_then(|height| indexes.height.day1.collect_one(height))
        })
}

fn mode_weights(
    day: Day1,
    age_supplies: &[&impl ReadableVec<Day1, Option<Sats>>],
    cointime_liveliness: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    coinflow_mobility: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    coinflow_spending_rate: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    bounds: &[AgeBand; AGE_COHORT_COUNT],
) -> ModeWeights {
    debug_assert_eq!(COINFLOW_HORIZON_START + HORIZON_COUNT, MODE_COUNT);
    let mut weights = [None; MODE_COUNT];
    weights[RAW_MODE] = Some([1.0; AGE_COHORT_COUNT]);
    weights[COINTIME_MODE] = collect_age_values(cointime_liveliness, age_supplies, day)
        .map(|values| values.map(|v| v.clamp(0.0, 1.0)));
    weights[COINFLOW_MODE] = collect_age_values(coinflow_mobility, age_supplies, day)
        .map(|values| values.map(|v| v.clamp(0.0, 1.0)));

    if let Some(hazards) = collect_age_values(coinflow_spending_rate, age_supplies, day) {
        let hazards = hazards.map(|value| value.max(0.0));
        for (offset, horizon) in HORIZON_DAYS.iter().copied().enumerate() {
            weights[COINFLOW_HORIZON_START + offset] = Some(std::array::from_fn(|age| {
                coinflow::horizon_mobility(&hazards, age, horizon, bounds)
            }));
        }
    }
    weights
}

fn collect_age_values<T>(
    sources: &[&impl ReadableVec<Day1, Option<T>>],
    supplies: &[&impl ReadableVec<Day1, Option<Sats>>],
    day: Day1,
) -> Option<[f64; AGE_COHORT_COUNT]>
where
    T: VecValue,
    f64: From<T>,
{
    if sources.len() != AGE_COHORT_COUNT || supplies.len() != AGE_COHORT_COUNT {
        return None;
    }

    let mut values = [0.0; AGE_COHORT_COUNT];
    for ((value, source), supply) in values.iter_mut().zip(sources).zip(supplies) {
        let supply = supply.collect_one(day).flatten()?;
        *value = resolve_age_value(source.collect_one(day).flatten(), supply)?;
    }
    Some(values)
}

fn resolve_age_value<T>(value: Option<T>, supply: Sats) -> Option<f64>
where
    f64: From<T>,
{
    match value.map(f64::from) {
        Some(value) if value.is_finite() => Some(value),
        _ if supply == Sats::ZERO => Some(0.0),
        _ => None,
    }
}

fn read_weighted_urpd(
    states_path: &Path,
    date: Date,
    weights: &ModeWeights,
) -> Result<Option<BTreeMap<CentsCompact, [f64; MODE_COUNT]>>> {
    let mut weighted = BTreeMap::<CentsCompact, [f64; MODE_COUNT]>::new();

    for (age, name) in AGE_RANGE_NAMES.iter().enumerate() {
        let cohort = CohortContext::Utxo.prefixed(name.id);
        let path = states_path.join(cohort).join("urpd").join(date.to_string());
        let bytes = fs::read(&path).map_err(|error| {
            std::io::Error::new(
                error.kind(),
                format!("Cannot read URPD '{}': {error}", path.display()),
            )
        })?;
        let urpd = UrpdRaw::deserialize(&bytes)?;

        for (price, sats) in urpd.map {
            let mass = u64::from(sats) as f64;
            let bucket = weighted.entry(price).or_insert([0.0; MODE_COUNT]);
            for (bucket, weights) in bucket.iter_mut().zip(weights) {
                if let Some(weights) = weights {
                    *bucket += mass * weights[age];
                }
            }
        }
    }

    Ok((!weighted.is_empty()).then_some(weighted))
}

fn evaluate_day(
    weighted: &BTreeMap<CentsCompact, [f64; MODE_COUNT]>,
    thresholds: &Thresholds,
    result: &mut DayResult,
) {
    let mut total_mass = [0.0; MODE_COUNT];
    let mut has_positive_cost = [false; MODE_COUNT];

    for (price, buckets) in weighted {
        for mode in 0..MODE_COUNT {
            let mass = buckets[mode];
            total_mass[mode] += mass;
            has_positive_cost[mode] |= price.inner() != 0 && mass > 0.0;
        }
    }

    for mode in 0..MODE_COUNT {
        let denominator = total_mass[mode];
        let Some(thresholds) = thresholds[mode] else {
            continue;
        };
        if denominator <= 0.0 || !has_positive_cost[mode] {
            continue;
        }

        let mut remaining_loss = denominator;
        let mut floors = [Cents::NAN; PERCENTILE_COUNT];
        let mut p95_floor = None;
        for (price, buckets) in weighted {
            remaining_loss -= buckets[mode];
            let remaining_share = remaining_loss / denominator;
            for percentile in 0..PERCENTILE_COUNT {
                if floors[percentile].is_nan() && remaining_share <= thresholds[percentile] {
                    floors[percentile] = Cents::from(*price);
                    if percentile == 0 {
                        p95_floor = Some(*price);
                    }
                }
            }
            if floors.iter().all(|floor| !floor.is_nan()) {
                break;
            }
        }
        result.floor[mode] = floors;
        if let Some(p95_floor) = p95_floor {
            result.level[mode] = conditional_levels(weighted, mode, p95_floor);
        }
    }
}

fn conditional_levels(
    weighted: &BTreeMap<CentsCompact, [f64; MODE_COUNT]>,
    mode: usize,
    lower: CentsCompact,
) -> [Cents; LEVEL_COUNT] {
    let mut levels = [Cents::NAN; LEVEL_COUNT];
    let total = weighted
        .range(lower..)
        .map(|(_, buckets)| buckets[mode])
        .filter(|mass| mass.is_finite() && *mass > 0.0)
        .sum::<f64>();
    if !total.is_finite() || total <= 0.0 {
        return levels;
    }

    let mut cumulative = 0.0;
    let mut percentile = 0;
    for (price, buckets) in weighted.range(lower..) {
        let mass = buckets[mode];
        if !mass.is_finite() || mass <= 0.0 {
            continue;
        }
        cumulative += mass;
        while percentile < LEVEL_COUNT && cumulative >= total * LEVEL_PERCENTILES[percentile] {
            levels[percentile] = Cents::from(*price);
            percentile += 1;
        }
        if percentile == LEVEL_COUNT {
            break;
        }
    }
    levels
}

fn quantile(sorted: &[f64], percentile: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    let position = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    Some(sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction)
}

fn insert_sorted(values: &mut Vec<f64>, value: f64) {
    let index = values
        .binary_search_by(|candidate| candidate.partial_cmp(&value).unwrap_or(Ordering::Less))
        .unwrap_or_else(|index| index);
    values.insert(index, value);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantile_linearly_interpolates() {
        assert_eq!(quantile(&[0.0, 1.0], 0.95), Some(0.95));
        assert_eq!(quantile(&[], 0.95), None);
    }

    #[test]
    fn empty_age_cohort_uses_zero_weight() {
        assert_eq!(resolve_age_value::<StoredF64>(None, Sats::ZERO), Some(0.0));
        assert_eq!(
            resolve_age_value(Some(StoredF64::NAN), Sats::ZERO),
            Some(0.0)
        );
    }

    #[test]
    fn non_empty_age_cohort_requires_finite_weight() {
        let supply = Sats::from(1_u64);
        assert_eq!(resolve_age_value::<StoredF64>(None, supply), None);
        assert_eq!(resolve_age_value(Some(StoredF64::NAN), supply), None);
        assert_eq!(
            resolve_age_value(Some(StoredF64::from(0.25)), supply),
            Some(0.25)
        );
    }

    #[test]
    fn daily_loss_share_calibrates_the_floor() {
        let weighted = BTreeMap::from([
            (CentsCompact::new(100), [50.0; MODE_COUNT]),
            (CentsCompact::new(200), [50.0; MODE_COUNT]),
        ]);
        let mut calibration = Calibration {
            histories: std::array::from_fn(|_| vec![0.5; MIN_CALIBRATION_DAYS]),
        };
        let shares = [Some(0.5); MODE_COUNT];
        let thresholds = calibration.thresholds(&shares);
        let mut result = DayResult::from_thresholds(&thresholds);
        evaluate_day(&weighted, &thresholds, &mut result);
        calibration.observe(shares);

        assert_eq!(
            result.loss_threshold[COINFLOW_MODE],
            [StoredF64::from(0.5); PERCENTILE_COUNT]
        );
        assert_eq!(
            result.floor[COINFLOW_MODE],
            [Cents::new(100); PERCENTILE_COUNT]
        );
        assert_eq!(
            result.level[COINFLOW_MODE],
            [
                Cents::new(100),
                Cents::new(100),
                Cents::new(100),
                Cents::new(100),
                Cents::new(100),
                Cents::new(200),
                Cents::new(200),
                Cents::new(200),
                Cents::new(200),
            ]
        );
        assert_eq!(
            calibration.histories[COINFLOW_MODE].len(),
            MIN_CALIBRATION_DAYS + 1
        );
    }

    #[test]
    fn zero_cost_distribution_stays_missing() {
        let weighted = BTreeMap::from([(CentsCompact::new(0), [100.0; MODE_COUNT])]);
        let mut calibration = Calibration {
            histories: std::array::from_fn(|_| vec![0.5; MIN_CALIBRATION_DAYS]),
        };
        let shares = [Some(1.0); MODE_COUNT];
        let thresholds = calibration.thresholds(&shares);
        let mut result = DayResult::from_thresholds(&thresholds);
        evaluate_day(&weighted, &thresholds, &mut result);
        calibration.observe(shares);

        assert_eq!(result.loss_threshold[RAW_MODE][0], StoredF64::from(0.5));
        assert!(result.floor[RAW_MODE][0].is_nan());
        assert_eq!(
            calibration.histories[RAW_MODE].len(),
            MIN_CALIBRATION_DAYS + 1
        );
    }

    #[test]
    fn missing_framework_share_does_not_update_history() {
        let mut calibration = Calibration {
            histories: std::array::from_fn(|_| Vec::new()),
        };
        let shares = [None; MODE_COUNT];
        let thresholds = calibration.thresholds(&shares);
        calibration.observe(shares);

        assert_eq!(thresholds, [None; MODE_COUNT]);
        assert!(calibration.histories[RAW_MODE].is_empty());
    }
}
