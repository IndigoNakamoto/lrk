use brk_traversable::Traversable;
use brk_types::{CapitalSentimentPhase, StoredBool, StoredI8, StoredU8};
use vecdb::{Rw, StorageMode};

use crate::internal::{DailyMetric, LazyDailyMetric};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Compact daily source of truth.
    #[traversable(hidden)]
    pub(super) phase_code: DailyMetric<StoredU8, M>,

    /// BRK Signal position: `true` is long and `false` is short.
    pub is_long: DailyMetric<StoredBool, M>,
    /// Lazy complement of `is_long`.
    pub is_short: LazyDailyMetric<StoredBool, StoredBool>,
    pub phase: LazyDailyMetric<Option<CapitalSentimentPhase>, StoredU8>,
    pub score: LazyDailyMetric<Option<StoredI8>, Option<CapitalSentimentPhase>>,
}
