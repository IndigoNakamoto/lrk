use brk_traversable::Traversable;
use brk_types::StoredU32;
use vecdb::{Rw, StorageMode};

use crate::internal::{PerBlockCumulativeRolling, ValuePerBlockCumulativeRolling};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Count of HogEx integration txs per block (`is_hog_ex`).
    pub tx_count: PerBlockCumulativeRolling<StoredU32, M>,
    /// Per-block sum of per-tx input values including MWEB prevouts.
    pub raw_input_volume: ValuePerBlockCumulativeRolling<M>,
}
