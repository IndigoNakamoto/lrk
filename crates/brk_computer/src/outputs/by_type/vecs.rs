use brk_cohort::ByType;
use brk_traversable::Traversable;
use brk_types::{PartsPerMillion32, StoredU64};
use vecdb::{Rw, StorageMode};

use super::{CachedSpendableOutputCount, WithOutputTypes};
use crate::internal::{
    CachedCountPerBlockCumulativeRolling, LazyPercentCumulativeRolling, PerBlockCumulativeRolling,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub output_count: WithOutputTypes<CachedCountPerBlockCumulativeRolling<M>>,
    pub spendable_output_count: CachedSpendableOutputCount,
    pub output_share: ByType<LazyPercentCumulativeRolling<PartsPerMillion32>>,
    pub tx_count: WithOutputTypes<PerBlockCumulativeRolling<StoredU64, M>>,
    pub tx_share: ByType<LazyPercentCumulativeRolling<PartsPerMillion32>>,
}
