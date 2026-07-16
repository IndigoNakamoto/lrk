use brk_traversable::Traversable;
use brk_types::{VSize, Weight};
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerTxDistribution, LazyPerTxDistributionTransformed};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub vsize: LazyPerTxDistributionTransformed<VSize, Weight, Weight>,
    pub weight: LazyPerTxDistribution<Weight, Weight, M>,
}
