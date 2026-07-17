use brk_traversable::Traversable;
use brk_types::{VSize, Weight};
use vecdb::{Rw, StorageMode};

use crate::internal::{LazyPerTxDistributionTransformed, TxDerivedDistribution};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub vsize: LazyPerTxDistributionTransformed<VSize, Weight, Weight>,
    pub weight: TxDerivedDistribution<Weight, M>,
}
