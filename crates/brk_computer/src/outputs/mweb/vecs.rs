use brk_traversable::Traversable;
use vecdb::{Rw, StorageMode};

use crate::internal::{ValuePerBlock, ValuePerBlockCumulative};

/// Litecoin MWEB peg accounting derived purely from the canonical chain.
///
/// MWEB outputs (peg-pool witness v8 + peg-in witness v9) are unspendable and
/// excluded from the transparent UTXO set, so they never appear in the
/// spendable circulating supply. The net pegged balance is recovered as
/// `cumulative(outputs_value) - cumulative(inputs_value)`, which telescopes to
/// the value of the currently-unspent MWEB outputs (i.e. the peg-pool balance).
#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    /// Gross value of MWEB outputs created per block (peg-ins + new peg-pool).
    pub outputs_value: ValuePerBlockCumulative<M>,
    /// Gross value of MWEB outputs spent per block (consumed peg-ins + prior peg-pool).
    pub inputs_value: ValuePerBlockCumulative<M>,
    /// Net LTC pegged into MWEB, market-priced. Included in total supply as a
    /// distinct opaque bucket (no age/realized data is possible for in-MWEB coins).
    pub balance: ValuePerBlock<M>,
}
