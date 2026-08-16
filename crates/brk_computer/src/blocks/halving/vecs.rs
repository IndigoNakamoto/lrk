use brk_traversable::Traversable;
use brk_types::{Halving, StoredF32, StoredU32};

use crate::internal::LazyPerBlock;

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub epoch: LazyPerBlock<Halving>,
    pub blocks_to_halving: LazyPerBlock<StoredU32>,
    pub days_to_halving: LazyPerBlock<StoredF32, StoredU32>,
}
