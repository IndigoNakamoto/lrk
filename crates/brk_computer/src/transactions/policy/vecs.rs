use brk_traversable::Traversable;
use brk_types::{StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::PerBlockCumulativeRolling;

/// Transactions that do not satisfy the standard relay policy at their height.
#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    pub nonstandard: PerBlockCumulativeRolling<StoredU64, M>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: CountVecs<M>,
    pub is_nonstandard: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
}
