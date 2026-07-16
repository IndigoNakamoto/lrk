use brk_traversable::Traversable;
use brk_types::{FeeRate, Height, Sats, StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::PerTxDistribution;

#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    pub cpfp_parent: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
    pub cpfp_child: M::Stored<EagerVec<PcoVec<Height, StoredU64>>>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: CountVecs<M>,
    pub input_value: M::Stored<EagerVec<PcoVec<TxIndex, Sats>>>,
    pub output_value: M::Stored<EagerVec<PcoVec<TxIndex, Sats>>>,
    pub fee: PerTxDistribution<Sats, M>,
    pub fee_rate: M::Stored<EagerVec<PcoVec<TxIndex, FeeRate>>>,
    pub effective_fee_rate: PerTxDistribution<FeeRate, M>,
    pub is_cpfp_parent: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
    pub is_cpfp_child: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
}
