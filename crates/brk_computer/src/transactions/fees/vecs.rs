use brk_traversable::Traversable;
use brk_types::{FeeRate, Sats, StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::{PerBlockCumulativeRolling, PerTxDistribution};

/// Confirmed transactions participating in same-block CPFP clusters.
#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    pub cpfp_parent: PerBlockCumulativeRolling<StoredU64, M>,
    pub cpfp_child: PerBlockCumulativeRolling<StoredU64, M>,
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
