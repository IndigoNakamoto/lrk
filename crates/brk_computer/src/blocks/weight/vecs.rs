use brk_traversable::Traversable;
use brk_types::{PartsPerMillion32, StoredU64, Weight};

use crate::internal::{LazyPerBlockRolling, LazyPercentVec};

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub weight: LazyPerBlockRolling<Weight, StoredU64>,
    pub fullness: LazyPercentVec<PartsPerMillion32, Weight>,
}
