use std::collections::VecDeque;

use brk_error::Result;
use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{Bitcoin, Dollars, Height, PartsPerMillion32, StoredF32, StoredI8, Version};
use schemars::JsonSchema;
use vecdb::{
    AnyStoredVec, AnyVec, Database, Exit, ReadableVec, Rw, StorageMode, VecIndex, WritableVec,
};

use crate::{
    indexes,
    internal::{
        NumericValue, PerBlock, PercentPerBlock, algo::FenwickTree,
        db_utils::validate_any_computed_version_or_reset,
    },
};

const VERSION: Version = Version::new(4);
const MIN_HISTORY_BLOCKS: usize = 210_000;
const WRITE_INTERVAL: usize = 10_000;
const BANDS: [(f64, i8); 3] = [(0.00025, 3), (0.0005, 2), (0.001, 1)];

#[derive(Clone, Copy)]
struct Config {
    upper_tail: bool,
    rolling: bool,
    positive_only: bool,
}

const REALIZED: Config = Config {
    upper_tail: true,
    rolling: false,
    positive_only: false,
};

const COINS_IN_LOSS: Config = Config {
    upper_tail: true,
    rolling: false,
    positive_only: true,
};

const SELLER_EXHAUSTION: Config = Config {
    upper_tail: false,
    rolling: true,
    positive_only: true,
};

/// Historical extremeness of one metric.
///
/// `tail` is the current observation's top- or bottom-tail share, `threshold`
/// is the highlight boundary, and `rank` is 0 through 3.
#[derive(Traversable)]
pub struct Extreme<T, M: StorageMode = Rw>
where
    T: NumericValue + JsonSchema,
{
    pub threshold: PerBlock<T, M>,
    pub tail: PercentPerBlock<PartsPerMillion32, M>,
    pub rank: PerBlock<StoredI8, M>,
}

impl<T> Extreme<T>
where
    T: NumericValue + JsonSchema,
{
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            threshold: PerBlock::forced_import(db, &format!("{name}_threshold"), version, indexes)?,
            tail: PercentPerBlock::forced_import(db, &format!("{name}_tail"), version, indexes)?,
            rank: PerBlock::forced_import(db, &format!("{name}_rank"), version, indexes)?,
        })
    }

    fn compute(
        &mut self,
        indexer: &Indexer,
        source: &impl ReadableVec<Height, T>,
        config: Config,
        exit: &Exit,
    ) -> Result<()> {
        let dependency_version = source.version();
        for output in [
            &mut self.threshold.height as &mut dyn AnyStoredVec,
            &mut self.tail.ppm.height,
            &mut self.rank.height,
        ] {
            validate_any_computed_version_or_reset(output, dependency_version)?;
        }

        let source_end = source.len();
        let start = [
            self.threshold.height.len(),
            self.tail.ppm.height.len(),
            self.rank.height.len(),
            indexer.safe_lengths().height.to_usize(),
            source_end,
        ]
        .into_iter()
        .min()
        .unwrap_or_default();

        self.threshold.height.any_truncate_if_needed_at(start)?;
        self.tail.ppm.height.any_truncate_if_needed_at(start)?;
        self.rank.height.any_truncate_if_needed_at(start)?;

        let values: Vec<f64> = source
            .collect_range_at(0, source_end)
            .into_iter()
            .map(Into::into)
            .collect();
        let is_valid = |value: f64| value.is_finite() && (!config.positive_only || value > 0.0);
        let mut coordinates: Vec<f64> = values.iter().copied().filter(|&v| is_valid(v)).collect();
        coordinates.sort_unstable_by(f64::total_cmp);
        coordinates.dedup_by(|a, b| a.total_cmp(b).is_eq());

        let mut history = History::new(coordinates.len().max(1), config.rolling);
        for &value in &values[..start] {
            if is_valid(value) {
                history.add(bucket(&coordinates, value));
            }
        }

        for (height_index, &value) in values.iter().enumerate().skip(start) {
            let state = if is_valid(value) && history.len >= MIN_HISTORY_BLOCKS {
                event_state(value, &coordinates, &history, config)
            } else {
                EventState::missing()
            };

            self.threshold.height.push(T::from(state.threshold));
            self.tail
                .ppm
                .height
                .push(PartsPerMillion32::from(state.tail));
            self.rank.height.push(StoredI8::new(state.rank));

            if is_valid(value) {
                history.add(bucket(&coordinates, value));
            }

            if (height_index + 1).is_multiple_of(WRITE_INTERVAL) || height_index + 1 == source_end {
                let _lock = exit.lock();
                self.threshold.height.write()?;
                self.tail.ppm.height.write()?;
                self.rank.height.write()?;
            }
        }

        Ok(())
    }
}

#[derive(Traversable)]
pub struct Extremes<M: StorageMode = Rw> {
    pub coins_in_loss: Extreme<Bitcoin, M>,
    pub profit_taking: Extreme<Dollars, M>,
    pub capitulation: Extreme<Dollars, M>,
    pub peak_regret: Extreme<Dollars, M>,
    pub seller_exhaustion: Extreme<StoredF32, M>,
}

impl Extremes {
    pub(super) fn forced_import(
        db: &Database,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let version = parent_version + VERSION;
        Ok(Self {
            coins_in_loss: Extreme::forced_import(
                db,
                "rarity_meter_coins_in_loss",
                version,
                indexes,
            )?,
            profit_taking: Extreme::forced_import(
                db,
                "rarity_meter_profit_taking",
                version,
                indexes,
            )?,
            capitulation: Extreme::forced_import(
                db,
                "rarity_meter_capitulation",
                version,
                indexes,
            )?,
            peak_regret: Extreme::forced_import(db, "rarity_meter_peak_regret", version, indexes)?,
            seller_exhaustion: Extreme::forced_import(
                db,
                "rarity_meter_seller_exhaustion",
                version,
                indexes,
            )?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn compute(
        &mut self,
        indexer: &Indexer,
        coins_in_loss: &impl ReadableVec<Height, Bitcoin>,
        realized_profit: &impl ReadableVec<Height, Dollars>,
        realized_loss: &impl ReadableVec<Height, Dollars>,
        peak_regret: &impl ReadableVec<Height, Dollars>,
        seller_exhaustion: &impl ReadableVec<Height, StoredF32>,
        exit: &Exit,
    ) -> Result<()> {
        self.coins_in_loss
            .compute(indexer, coins_in_loss, COINS_IN_LOSS, exit)?;
        self.profit_taking
            .compute(indexer, realized_profit, REALIZED, exit)?;
        self.capitulation
            .compute(indexer, realized_loss, REALIZED, exit)?;
        self.peak_regret
            .compute(indexer, peak_regret, REALIZED, exit)?;
        self.seller_exhaustion
            .compute(indexer, seller_exhaustion, SELLER_EXHAUSTION, exit)?;
        Ok(())
    }
}

struct History {
    tree: FenwickTree<f64>,
    len: usize,
    rolling: Option<VecDeque<usize>>,
}

impl History {
    fn new(size: usize, rolling: bool) -> Self {
        Self {
            tree: FenwickTree::new(size),
            len: 0,
            rolling: rolling.then(VecDeque::new),
        }
    }

    fn add(&mut self, bucket: usize) {
        self.tree.add(bucket, &1.0);
        self.len += 1;

        if let Some(rolling) = &mut self.rolling {
            rolling.push_back(bucket);
            if rolling.len() > MIN_HISTORY_BLOCKS {
                let expired = rolling.pop_front().unwrap();
                self.tree.add(expired, &-1.0);
                self.len -= 1;
            }
        }
    }

    fn quantile(&self, coordinates: &[f64], percentile: f64) -> f64 {
        let target = ((self.len - 1) as f64 * percentile).floor();
        let mut index = [0];
        self.tree.kth(&[target], &|count: &f64| *count, &mut index);
        coordinates[index[0]]
    }

    fn tail(&self, bucket: usize, upper: bool) -> f64 {
        let less = if bucket == 0 {
            0.0
        } else {
            self.tree.prefix_sum(bucket - 1)
        };
        let less_or_equal = self.tree.prefix_sum(bucket);
        let count = if upper {
            self.len as f64 - less
        } else {
            less_or_equal
        };
        (count + 1.0) / (self.len as f64 + 1.0)
    }
}

struct EventState {
    threshold: f64,
    tail: f64,
    rank: i8,
}

impl EventState {
    fn missing() -> Self {
        Self {
            threshold: f64::NAN,
            tail: f64::NAN,
            rank: 0,
        }
    }
}

fn event_state(value: f64, coordinates: &[f64], history: &History, config: Config) -> EventState {
    let percentile = |tail: f64| {
        if config.upper_tail { 1.0 - tail } else { tail }
    };
    let threshold = history.quantile(coordinates, percentile(BANDS[0].0));
    let rank = BANDS
        .into_iter()
        .find_map(|(tail, rank)| {
            let boundary = history.quantile(coordinates, percentile(tail));
            let reached = if config.upper_tail {
                value >= boundary
            } else {
                value <= boundary
            };
            reached.then_some(rank)
        })
        .unwrap_or_default();

    EventState {
        threshold,
        tail: history.tail(bucket(coordinates, value), config.upper_tail),
        rank,
    }
}

fn bucket(coordinates: &[f64], value: f64) -> usize {
    coordinates
        .binary_search_by(|candidate| candidate.total_cmp(&value))
        .expect("event value must exist in coordinate set")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn history(values: &[f64]) -> (Vec<f64>, History) {
        let mut coordinates = values.to_vec();
        coordinates.sort_unstable_by(f64::total_cmp);
        coordinates.dedup_by(|a, b| a.total_cmp(b).is_eq());
        let mut history = History::new(coordinates.len(), false);
        for &value in values {
            history.add(bucket(&coordinates, value));
        }
        (coordinates, history)
    }

    #[test]
    fn upper_tail_includes_current_observation() {
        let (mut coordinates, mut history) = history(&(1..=100).map(f64::from).collect::<Vec<_>>());
        coordinates.push(101.0);
        history.tree = {
            let mut tree = FenwickTree::new(coordinates.len());
            for value in 1..=100 {
                tree.add(bucket(&coordinates, f64::from(value)), &1.0);
            }
            tree
        };

        let state = event_state(101.0, &coordinates, &history, REALIZED);
        assert!((state.tail - 1.0 / 101.0).abs() < f64::EPSILON);
        assert_eq!(state.rank, 3);
        assert_eq!(state.threshold, 99.0);
    }

    #[test]
    fn lower_tail_includes_current_observation() {
        let coordinates: Vec<_> = (0..=100).map(f64::from).collect();
        let mut history = History::new(coordinates.len(), false);
        for value in 1..=100 {
            history.add(bucket(&coordinates, f64::from(value)));
        }
        let state = event_state(0.0, &coordinates, &history, SELLER_EXHAUSTION);

        assert!((state.tail - 1.0 / 101.0).abs() < f64::EPSILON);
        assert_eq!(state.rank, 3);
        assert_eq!(state.threshold, 1.0);
    }
}
