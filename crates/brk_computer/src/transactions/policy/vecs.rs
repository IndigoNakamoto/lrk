use brk_traversable::Traversable;
use brk_types::{Height, StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
    pub is_nonstandard: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
}
