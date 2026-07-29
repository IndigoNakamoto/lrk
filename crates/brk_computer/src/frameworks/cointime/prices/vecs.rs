use brk_traversable::Traversable;
use vecdb::{Rw, StorageMode};

use crate::internal::PriceWithRatioPerBlock;

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub vaulted: PriceWithRatioPerBlock<M>,
    pub active: PriceWithRatioPerBlock<M>,
    pub true_market_mean: PriceWithRatioPerBlock<M>,
    pub cointime: PriceWithRatioPerBlock<M>,
}
