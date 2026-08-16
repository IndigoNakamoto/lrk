use brk_traversable::Traversable;
use brk_types::{StoredBool, StoredU64, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::PerBlockCumulativeRolling;

/// Transaction counts by detected structural pattern.
///
/// These are heuristic classifications of transactions, not protocol labels.
#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    /// CoinJoin candidates with repeated output values and no address reuse.
    pub coinjoin: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions with at least five times as many inputs as outputs.
    pub consolidation: PerBlockCumulativeRolling<StoredU64, M>,
    /// Non-coinbase transactions with at least five times as many outputs as inputs.
    pub batch_payout: PerBlockCumulativeRolling<StoredU64, M>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: CountVecs<M>,
    pub is_coinjoin: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
    pub is_consolidation: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
    pub is_batch_payout: M::Stored<EagerVec<PcoVec<TxIndex, StoredBool>>>,
}
