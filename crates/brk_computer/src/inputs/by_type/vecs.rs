use brk_cohort::SpendableType;
use brk_traversable::Traversable;
use brk_types::{PartsPerMillion32, StoredU64};
use vecdb::{Rw, StorageMode};

use super::WithInputTypes;
use crate::internal::{
    CachedCountPerBlockCumulativeRolling, LazyPercentCumulativeRolling, PerBlockCumulativeRolling,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub input_count: WithInputTypes<CachedCountPerBlockCumulativeRolling<M>>,
    pub input_share: SpendableType<LazyPercentCumulativeRolling<PartsPerMillion32>>,
    pub tx_count: WithInputTypes<PerBlockCumulativeRolling<StoredU64, M>>,
    pub tx_share: SpendableType<LazyPercentCumulativeRolling<PartsPerMillion32>>,
}
