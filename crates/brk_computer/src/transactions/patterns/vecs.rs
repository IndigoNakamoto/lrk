use brk_traversable::Traversable;
use brk_types::{Height, StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    pub coinjoin: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
    pub consolidation: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
    pub batch_payout: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: CountVecs<M>,
    pub is_coinjoin: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
    pub is_consolidation: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
    pub is_batch_payout: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
}
