use brk_traversable::Traversable;
use brk_types::{FeeRate, Sats, TxIndex};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::PerTxDistribution;

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub input_value: M::Stored<EagerVec<PcoVec<TxIndex, Sats>>>,
    /// Per-tx input value excluding inputs that spend Litecoin MWEB prevouts
    /// (peg-pool / peg-in outputs). Drives `transfer_volume` so the HogEx
    /// pool re-spend and peg-in aggregation don't inflate economic volume.
    /// Coinbase txs keep the `Sats::MAX` sentinel so they stay excluded.
    pub transfer_input_value: M::Stored<EagerVec<PcoVec<TxIndex, Sats>>>,
    pub output_value: M::Stored<EagerVec<PcoVec<TxIndex, Sats>>>,
    pub fee: PerTxDistribution<Sats, M>,
    pub fee_rate: M::Stored<EagerVec<PcoVec<TxIndex, FeeRate>>>,
    pub effective_fee_rate: PerTxDistribution<FeeRate, M>,
}
