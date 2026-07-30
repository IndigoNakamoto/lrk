use brk_traversable::Traversable;
use brk_types::{Epoch, PartsPerMillionSigned32, StoredF32, StoredF64, StoredU32};

use crate::internal::{LazyPerBlock, LazyPercentPerBlock, Resolutions};

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub value: Resolutions<StoredF64>,
    pub hashrate: LazyPerBlock<StoredF64>,
    pub adjustment: LazyPercentPerBlock<PartsPerMillionSigned32>,
    pub epoch: LazyPerBlock<Epoch>,
    pub blocks_to_retarget: LazyPerBlock<StoredU32>,
    pub days_to_retarget: LazyPerBlock<StoredF32, StoredU32>,
}
