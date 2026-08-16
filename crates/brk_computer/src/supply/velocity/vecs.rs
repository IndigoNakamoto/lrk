use brk_traversable::Traversable;
use brk_types::StoredF64;

use crate::internal::LazyPerBlock;

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub native: LazyPerBlock<StoredF64>,
    pub fiat: LazyPerBlock<StoredF64>,
}
