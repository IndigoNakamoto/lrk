use brk_traversable::Traversable;
use brk_types::StoredU64;

use crate::internal::{ConstantVecs, LazyPerBlockCumulativeRolling, Windows};

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub target: Windows<ConstantVecs<StoredU64>>,
    pub total: LazyPerBlockCumulativeRolling<StoredU64>,
}
