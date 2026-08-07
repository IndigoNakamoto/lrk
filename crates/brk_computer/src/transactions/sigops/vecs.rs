use brk_traversable::Traversable;
use brk_types::StoredU64;
use vecdb::{Rw, StorageMode};

use crate::internal::PerBlockCumulativeRolling;

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Total BIP-141 sigop cost, not a raw opcode count.
    pub total: PerBlockCumulativeRolling<StoredU64, M>,
}
