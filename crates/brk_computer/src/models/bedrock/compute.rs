use std::{cmp::Ordering, collections::BTreeMap, fs, path::Path};

use brk_cohort::{AGE_RANGE_FILTERS, AGE_RANGE_NAMES, CohortContext, TERM_FILTERS, UTXO_ALL_NAME};
use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Cents, CentsCompact, Date, Day1, Sats, StoredF64, UrpdRaw, UrpdWeight, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecValue, WritableVec};

use super::{
    STORED_URPD_COHORTS,
    vecs::{Levels, MODE_COUNT, MODE_NAMES, ModeVecs, Percentiles, Vecs},
    weighted_urpd_name,
};
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
const WEIGHTED_MODE_COUNT: usize = MODE_COUNT - 1;
const STORED_WEIGHT_COUNT: usize = UrpdWeight::WEIGHTED.len();
const PERCENTILES: [f64; PERCENTILE_COUNT] = [0.95, 0.98, 0.99, 0.995, 0.999];
const LEVEL_PERCENTILES: [f64; LEVEL_COUNT] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];
const WEIGHTED_URPD_VERSION: Version = Version::TWO;
const WEIGHTED_URPD_VERSION_FILE: &str = "bedrock_urpd.version";

const RAW_MODE: usize = 0;
const COINTIME_MODE: usize = 1;
const COINFLOW_MODE: usize = 2;
const COINFLOW_HORIZON_START: usize = 3;
const STH_TERM: usize = 0;
const LTH_TERM: usize = 1;
const TERM_COUNT: usize = 2;

type Thresholds = [Option<[f64; PERCENTILE_COUNT]>; MODE_COUNT];
type ModeWeights = [Option<[f64; AGE_COHORT_COUNT]>; MODE_COUNT];
type AllWeightedUrpds = [UrpdRaw; WEIGHTED_MODE_COUNT];
type TermWeightedUrpds = [[UrpdRaw; STORED_WEIGHT_COUNT]; TERM_COUNT];
type WeightedUrpdNames = [[String; STORED_WEIGHT_COUNT]; STORED_URPD_COHORTS.len()];

struct WeightedMasses {
    all: [f64; WEIGHTED_MODE_COUNT],
    terms: [[f64; STORED_WEIGHT_COUNT]; TERM_COUNT],
}

impl Default for WeightedMasses {
    fn default() -> Self {
        Self {
            all: [0.0; WEIGHTED_MODE_COUNT],
            terms: [[0.0; STORED_WEIGHT_COUNT]; TERM_COUNT],
        }
    }
}

type WeightedUrpd = BTreeMap<CentsCompact, WeightedMasses>;

struct DayUrpds {
    raw: UrpdRaw,
    all: AllWeightedUrpds,
    terms: TermWeightedUrpds,
}

impl DayUrpds {
    fn mode(&self, mode: usize) -> &UrpdRaw {
        if mode == RAW_MODE {
            &self.raw
        } else {
            &self.all[mode - 1]
        }
    }
}

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
        let cointime_wakefulness: Vec<_> = cointime
            .age_range
            .iter()
            .map(|cohort| &cohort.wakefulness.day1)
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
            &coinflow.all.supply_in_loss_share.day1,
        ]
        .into_iter()
        .chain(
            coinflow
                .all
                .horizon
                .iter()
                .map(|horizon| &horizon.supply_in_loss_share.day1),
        )
        .collect();
        debug_assert_eq!(weighted_loss_shares.len(), MODE_COUNT - 1);
        let weighted_urpd_names = weighted_urpd_names();

        let weighted_urpd_source_version: Version = std::iter::once(WEIGHTED_URPD_VERSION)
            .chain(std::iter::once(indexes.day1.date.version()))
            .chain(std::iter::once(distribution.supply_state.version()))
            .chain(age_supplies.iter().map(|vec| vec.version()))
            .chain(cointime_wakefulness.iter().map(|vec| vec.version()))
            .chain(coinflow_mobility.iter().map(|vec| vec.version()))
            .sum();
        let source_version: Version = std::iter::once(weighted_urpd_source_version)
            .chain(coinflow_spending_rate.iter().map(|vec| vec.version()))
            .chain(std::iter::once(raw_loss_share.version()))
            .chain(weighted_loss_shares.iter().map(|vec| vec.version()))
            .sum();

        for vec in self.stored_vecs_mut() {
            validate_any_computed_version_or_reset(vec, source_version)?;
        }

        let source_end = std::iter::once(indexes.day1.date.len())
            .chain(std::iter::once(raw_loss_share.len()))
            .chain(weighted_loss_shares.iter().map(|vec| vec.len()))
            .chain(age_supplies.iter().map(|vec| vec.len()))
            .chain(cointime_wakefulness.iter().map(|vec| vec.len()))
            .chain(coinflow_mobility.iter().map(|vec| vec.len()))
            .chain(coinflow_spending_rate.iter().map(|vec| vec.len()))
            .min()
            .unwrap_or_default();
        let recompute_from = recompute_day(indexer, indexes)
            .map(usize::from)
            .unwrap_or_default();
        let weighted_urpd_is_current =
            read_weighted_urpd_version(&self.states_path)? == Some(weighted_urpd_source_version);
        if !weighted_urpd_is_current {
            reset_weighted_urpds(&self.states_path, &weighted_urpd_names)?;
        }
        let weighted_urpd_start = if weighted_urpd_is_current {
            recompute_from
        } else {
            0
        };
        let start = self
            .minimum_len()
            .min(recompute_from)
            .min(weighted_urpd_start)
            .min(source_end);

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

            let needs_evaluation = thresholds.iter().any(Option::is_some);
            let needs_rebuild = !weighted_urpd_is_current || day_index >= recompute_from;
            if let Some(date) = indexes.day1.date.collect_one(day)
                && (needs_rebuild || needs_evaluation)
                && UrpdRaw::path(&distribution.states_path, UTXO_ALL_NAME.id, date).try_exists()?
            {
                let weights = mode_weights(
                    day,
                    &age_supplies,
                    &cointime_wakefulness,
                    &coinflow_mobility,
                    &coinflow_spending_rate,
                    &bounds,
                );
                let urpds = build_day_urpds(&distribution.states_path, date, &weights)?;
                if needs_rebuild {
                    write_weighted_day_urpds(
                        &self.states_path,
                        &weighted_urpd_names,
                        date,
                        &urpds,
                    )?;
                }
                if needs_evaluation {
                    evaluate_day(&urpds, &thresholds, &mut result);
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

        if !weighted_urpd_is_current {
            let _lock = exit.lock();
            fs::create_dir_all(&self.states_path)?;
            weighted_urpd_source_version
                .write(&self.states_path.join(WEIGHTED_URPD_VERSION_FILE))?;
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
    cointime_wakefulness: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    coinflow_mobility: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    coinflow_spending_rate: &[&impl ReadableVec<Day1, Option<StoredF64>>],
    bounds: &[AgeBand; AGE_COHORT_COUNT],
) -> ModeWeights {
    debug_assert_eq!(COINFLOW_HORIZON_START + HORIZON_COUNT, MODE_COUNT);
    let mut weights = [None; MODE_COUNT];
    weights[RAW_MODE] = Some([1.0; AGE_COHORT_COUNT]);
    weights[COINTIME_MODE] = collect_age_values(cointime_wakefulness, age_supplies, day)
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

pub(super) fn resolve_age_value<T>(value: Option<T>, supply: Sats) -> Option<f64>
where
    f64: From<T>,
{
    match value.map(f64::from) {
        Some(value) if value.is_finite() => Some(value),
        _ if supply == Sats::ZERO => Some(0.0),
        _ => None,
    }
}

fn read_weighted_urpd_version(states_path: &Path) -> Result<Option<Version>> {
    let path = states_path.join(WEIGHTED_URPD_VERSION_FILE);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(Version::try_from(path.as_path())?))
}

fn reset_weighted_urpds(states_path: &Path, weighted_names: &WeightedUrpdNames) -> Result<()> {
    for name in weighted_names.iter().flatten() {
        remove_urpd_dir(states_path, name)?;
    }
    for mode in &MODE_NAMES[COINFLOW_HORIZON_START..] {
        remove_urpd_dir(states_path, &format!("bedrock_{mode}"))?;
    }
    Ok(())
}

fn remove_urpd_dir(states_path: &Path, name: &str) -> Result<()> {
    let path = UrpdRaw::dir(states_path, name);
    match fs::remove_dir_all(&path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(std::io::Error::new(
            error.kind(),
            format!("Cannot reset URPD '{}': {error}", path.display()),
        )
        .into()),
    }
}

fn build_day_urpds(
    distribution_states_path: &Path,
    date: Date,
    weights: &ModeWeights,
) -> Result<DayUrpds> {
    let raw = UrpdRaw::read(distribution_states_path, UTXO_ALL_NAME.id, date)?;
    let mut weighted = WeightedUrpd::new();

    for (age, (name, filter)) in AGE_RANGE_NAMES
        .iter()
        .zip(AGE_RANGE_FILTERS.iter())
        .enumerate()
    {
        let cohort = CohortContext::Utxo.prefixed(name.id);
        let source = UrpdRaw::read(distribution_states_path, &cohort, date)?;
        let term = if TERM_FILTERS.short.includes(filter) {
            STH_TERM
        } else {
            LTH_TERM
        };

        for (price, sats) in source.map {
            let mass = u64::from(sats) as f64;
            let bucket = weighted.entry(price).or_default();
            for (mode, (all, weights)) in bucket.all.iter_mut().zip(&weights[1..]).enumerate() {
                if let Some(weights) = weights {
                    let weighted_mass = mass * weights[age];
                    *all += weighted_mass;
                    if mode < STORED_WEIGHT_COUNT {
                        bucket.terms[term][mode] += weighted_mass;
                    }
                }
            }
        }
    }

    Ok(finalize_day_urpds(raw, weighted))
}

fn finalize_day_urpds(raw: UrpdRaw, weighted: WeightedUrpd) -> DayUrpds {
    let mut all: AllWeightedUrpds = std::array::from_fn(|_| UrpdRaw::default());
    let mut terms: TermWeightedUrpds =
        std::array::from_fn(|_| std::array::from_fn(|_| UrpdRaw::default()));

    for (price, masses) in weighted {
        for (distribution, mass) in all.iter_mut().zip(masses.all) {
            let sats = floor_weighted_sats(mass);
            if sats != Sats::ZERO {
                distribution.map.insert(price, sats);
            }
        }
        for (distributions, masses) in terms.iter_mut().zip(masses.terms) {
            for (distribution, mass) in distributions.iter_mut().zip(masses) {
                let sats = floor_weighted_sats(mass);
                if sats != Sats::ZERO {
                    distribution.map.insert(price, sats);
                }
            }
        }
    }

    DayUrpds { raw, all, terms }
}

fn floor_weighted_sats(mass: f64) -> Sats {
    debug_assert!(mass.is_finite() && mass >= 0.0);
    Sats::from(mass.floor() as u64)
}

fn write_weighted_day_urpds(
    models_states_path: &Path,
    weighted_names: &WeightedUrpdNames,
    date: Date,
    urpds: &DayUrpds,
) -> Result<()> {
    let distributions = [
        &urpds.all[..STORED_WEIGHT_COUNT],
        &urpds.terms[STH_TERM],
        &urpds.terms[LTH_TERM],
    ];
    for (names, distributions) in weighted_names.iter().zip(distributions) {
        for (name, distribution) in names.iter().zip(distributions) {
            UrpdRaw::write(
                models_states_path,
                name,
                date,
                distribution.map.iter().map(|(&price, &sats)| (price, sats)),
            )?;
        }
    }
    Ok(())
}

fn weighted_urpd_names() -> WeightedUrpdNames {
    std::array::from_fn(|cohort| {
        std::array::from_fn(|mode| {
            weighted_urpd_name(UrpdWeight::WEIGHTED[mode], STORED_URPD_COHORTS[cohort].id)
        })
    })
}

fn evaluate_day(urpds: &DayUrpds, thresholds: &Thresholds, result: &mut DayResult) {
    for mode in 0..MODE_COUNT {
        let urpd = urpds.mode(mode);
        let denominator = urpd.map.values().copied().map(u64::from).sum::<u64>();
        let Some(thresholds) = thresholds[mode] else {
            continue;
        };
        if denominator == 0
            || !urpd
                .map
                .iter()
                .any(|(price, sats)| price.inner() != 0 && *sats != Sats::ZERO)
        {
            continue;
        }

        let mut remaining_loss = denominator;
        let mut floors = [Cents::NAN; PERCENTILE_COUNT];
        let mut p95_floor = None;
        for (price, sats) in &urpd.map {
            remaining_loss -= u64::from(*sats);
            let remaining_share = remaining_loss as f64 / denominator as f64;
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
            result.level[mode] = conditional_levels(urpd, p95_floor);
        }
    }
}

fn conditional_levels(urpd: &UrpdRaw, lower: CentsCompact) -> [Cents; LEVEL_COUNT] {
    let mut levels = [Cents::NAN; LEVEL_COUNT];
    let total = urpd
        .map
        .range(lower..)
        .map(|(_, sats)| u64::from(*sats))
        .sum::<u64>();
    if total == 0 {
        return levels;
    }

    let mut cumulative = 0_u64;
    let mut percentile = 0;
    for (price, sats) in urpd.map.range(lower..) {
        let sats = u64::from(*sats);
        if sats == 0 {
            continue;
        }
        cumulative += sats;
        while percentile < LEVEL_COUNT
            && cumulative as f64 >= total as f64 * LEVEL_PERCENTILES[percentile]
        {
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

    fn repeated_day_urpds<const N: usize>(entries: [(u32, u64); N]) -> DayUrpds {
        let map = entries
            .into_iter()
            .map(|(price, sats)| (CentsCompact::new(price), Sats::from(sats)))
            .collect::<BTreeMap<_, _>>();
        DayUrpds {
            raw: UrpdRaw { map: map.clone() },
            all: std::array::from_fn(|_| UrpdRaw { map: map.clone() }),
            terms: std::array::from_fn(|_| std::array::from_fn(|_| UrpdRaw { map: map.clone() })),
        }
    }

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
        let urpds = repeated_day_urpds([(100, 50), (200, 50)]);
        let mut calibration = Calibration {
            histories: std::array::from_fn(|_| vec![0.5; MIN_CALIBRATION_DAYS]),
        };
        let shares = [Some(0.5); MODE_COUNT];
        let thresholds = calibration.thresholds(&shares);
        let mut result = DayResult::from_thresholds(&thresholds);
        evaluate_day(&urpds, &thresholds, &mut result);
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
        let urpds = repeated_day_urpds([(0, 100)]);
        let mut calibration = Calibration {
            histories: std::array::from_fn(|_| vec![0.5; MIN_CALIBRATION_DAYS]),
        };
        let shares = [Some(1.0); MODE_COUNT];
        let thresholds = calibration.thresholds(&shares);
        let mut result = DayResult::from_thresholds(&thresholds);
        evaluate_day(&urpds, &thresholds, &mut result);
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

    #[test]
    fn weighted_sats_are_floored_after_summing() {
        let summed_mass = 0.6 + 0.6;
        assert_eq!(floor_weighted_sats(summed_mass), Sats::from(1_u64));
        assert_eq!(floor_weighted_sats(0.6), Sats::ZERO);

        let weighted = BTreeMap::from([(
            CentsCompact::new(100),
            WeightedMasses {
                all: [summed_mass; WEIGHTED_MODE_COUNT],
                terms: [[0.6; STORED_WEIGHT_COUNT]; TERM_COUNT],
            },
        )]);
        let urpds = finalize_day_urpds(UrpdRaw::default(), weighted);

        assert_eq!(
            urpds.all[COINTIME_MODE - 1].map[&CentsCompact::new(100)],
            Sats::from(1_u64)
        );
        assert!(
            !urpds.terms[STH_TERM][COINTIME_MODE - 1]
                .map
                .contains_key(&CentsCompact::new(100))
        );
    }

    #[test]
    fn stores_only_cointime_and_coinflow_for_all_sth_and_lth() {
        let root =
            std::env::temp_dir().join(format!("brk-bedrock-urpd-file-{}", std::process::id()));
        let distribution_states = root.join("distribution");
        let models_states = root.join("models");
        let date = Date::new(2026, 8, 4);
        let names = weighted_urpd_names();
        let expected = repeated_day_urpds([(100, 21), (200, 34)]);
        assert_eq!(names[0][0], "bedrock_cointime");
        assert_eq!(names[0][1], "bedrock_coinflow");
        assert_eq!(names[1][0], "bedrock_cointime_sth");
        assert_eq!(names[2][1], "bedrock_coinflow_lth");

        UrpdRaw::write(
            &distribution_states,
            UTXO_ALL_NAME.id,
            date,
            expected.raw.map.iter().map(|(&price, &sats)| (price, sats)),
        )
        .unwrap();
        write_weighted_day_urpds(&models_states, &names, date, &expected).unwrap();

        for (cohort, names) in names.iter().enumerate() {
            let expected_distributions: &[UrpdRaw] = match cohort {
                0 => &expected.all[..STORED_WEIGHT_COUNT],
                1 => &expected.terms[STH_TERM],
                2 => &expected.terms[LTH_TERM],
                _ => unreachable!(),
            };
            for (name, expected) in names.iter().zip(expected_distributions) {
                assert_eq!(
                    UrpdRaw::read(&models_states, name, date).unwrap().map,
                    expected.map
                );
            }
        }
        assert!(!UrpdRaw::path(&models_states, "bedrock_raw", date).exists());
        assert!(!UrpdRaw::path(&models_states, "bedrock_coinflow_8y", date).exists());

        UrpdRaw::write(
            &models_states,
            "bedrock_coinflow_8y",
            date,
            expected.raw.map.iter().map(|(&price, &sats)| (price, sats)),
        )
        .unwrap();
        reset_weighted_urpds(&models_states, &names).unwrap();
        assert!(UrpdRaw::path(&distribution_states, UTXO_ALL_NAME.id, date).exists());
        assert!(!UrpdRaw::path(&models_states, "bedrock_coinflow_8y", date).exists());
        assert!(
            names
                .iter()
                .flatten()
                .all(|name| !UrpdRaw::path(&models_states, name, date).exists())
        );

        std::fs::remove_dir_all(root).unwrap();
    }
}
