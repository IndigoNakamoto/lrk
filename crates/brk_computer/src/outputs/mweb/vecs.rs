use brk_traversable::Traversable;
use brk_types::StoredU64;
use vecdb::{Rw, StorageMode};

use crate::internal::{
    PerBlockCumulativeRolling, ValuePerBlock, ValuePerBlockCumulative,
    ValuePerBlockCumulativeRolling,
};

/// Peg flow for one MWEB output class (pool or peg-in).
#[derive(Traversable)]
pub struct PegFlow<M: StorageMode = Rw> {
    pub outputs_value: ValuePerBlockCumulative<M>,
    pub inputs_value: ValuePerBlockCumulative<M>,
    pub balance: ValuePerBlock<M>,
}

/// Litecoin MWEB peg accounting derived purely from the canonical chain.
///
/// MWEB outputs (peg-pool witness v8 + peg-in witness v9) are unspendable and
/// excluded from the transparent UTXO set. The net pegged balance is recovered as
/// `cumulative(outputs_value) - cumulative(inputs_value)`, which telescopes to
/// the value of the currently-unspent MWEB outputs (i.e. the peg-pool balance).
#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Gross value of MWEB outputs created per block (peg-pool + peg-in).
    pub outputs_value: ValuePerBlockCumulative<M>,
    /// Gross value of MWEB outputs spent per block (consumed peg-ins + prior peg-pool).
    pub inputs_value: ValuePerBlockCumulative<M>,
    /// Net LTC pegged into MWEB, market-priced (pool + peg-in combined).
    pub balance: ValuePerBlock<M>,
    /// Witness v8 / HogAddr macro balance only.
    pub peg_pool: PegFlow<M>,
    /// Witness v9 peg-in outputs only (~0 steady-state balance).
    pub pegin: PegFlow<M>,
    /// v9 outputs created per block.
    pub pegin_count: PerBlockCumulativeRolling<StoredU64, StoredU64, M>,
    /// Transparent vout value on HogEx txs (excludes v8/v9).
    pub pegout_value: ValuePerBlockCumulativeRolling<M>,
    /// Transparent vout count on HogEx txs (excludes v8/v9).
    pub pegout_count: PerBlockCumulativeRolling<StoredU64, StoredU64, M>,
}
