use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, PartsPerMillion64, Sats};
use vecdb::{EagerVec, PcoVec, Rw, StorageMode};

use crate::internal::{
    CachedValuePerBlockFull, LazyPercentCumulativeRolling, LazyPercentRollingWindows,
    ValuePerBlockCumulative, ValuePerBlockCumulativeRolling,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub coinbase: ValuePerBlockCumulativeRolling<M>,
    pub subsidy: ValuePerBlockCumulativeRolling<M>,
    pub fees: CachedValuePerBlockFull<M>,
    pub output_volume: M::Stored<EagerVec<PcoVec<Height, Sats>>>,
    pub unclaimed: ValuePerBlockCumulative<M>,
    #[traversable(wrap = "fees", rename = "dominance")]
    pub fee_dominance: LazyPercentCumulativeRolling<PartsPerMillion32>,
    #[traversable(wrap = "subsidy", rename = "dominance")]
    pub subsidy_dominance: LazyPercentCumulativeRolling<PartsPerMillion32>,
    #[traversable(wrap = "fees", rename = "to_subsidy")]
    pub fee_to_subsidy: LazyPercentRollingWindows<PartsPerMillion64>,
}
