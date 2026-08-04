use brk_traversable::Traversable;
use brk_types::{CapitalSentimentPhase, StoredBool, StoredI8, StoredU8};
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerBlock, PerBlock};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Compact, per-block source of truth.
    #[traversable(hidden)]
    pub(super) phase_code: PerBlock<StoredU8, M>,

    /// BRK Signal position: `true` is long and `false` is cash.
    pub is_long: PerBlock<StoredBool, M>,
    /// Lazy complement of `is_long`.
    pub is_short: LazyPerBlock<StoredBool>,
    pub phase: LazyPerBlock<Option<CapitalSentimentPhase>, StoredU8>,
    pub score: LazyPerBlock<Option<StoredI8>, Option<CapitalSentimentPhase>>,
}
