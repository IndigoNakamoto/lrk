use brk_traversable::Traversable;
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerSecondWindows, ValuePerBlockCumulativeRolling};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub transfer_volume: ValuePerBlockCumulativeRolling<M>,
    pub tx_per_sec: LazyPerSecondWindows,
}
