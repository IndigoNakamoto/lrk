use brk_traversable::Traversable;
use brk_types::{PartsPerMillion32, StoredU64, Weight};
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerBlockRolling, PercentVec};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub weight: LazyPerBlockRolling<Weight, StoredU64>,
    pub fullness: PercentVec<PartsPerMillion32, M>,
}
