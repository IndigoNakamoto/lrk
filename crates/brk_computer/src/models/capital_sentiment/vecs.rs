use brk_traversable::Traversable;
use brk_types::{CapitalSentimentPhase, StoredI8};
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerBlock, PerBlock};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Compact, per-block source of truth.
    #[traversable(hidden)]
    pub(crate) phase_code: PerBlock<StoredI8, M>,

    pub phase: LazyPerBlock<Option<CapitalSentimentPhase>, StoredI8>,
    pub score: LazyPerBlock<Option<StoredI8>, Option<CapitalSentimentPhase>>,
}
