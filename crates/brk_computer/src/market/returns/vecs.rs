use brk_traversable::Traversable;
use brk_types::PartsPerMillionSigned64;
use vecdb::{Rw, StorageMode};

use crate::{
    internal::{LazyPercentPerBlock, StdDevPerBlock, Windows},
    investing::ByDcaCagr,
    market::lookback::ByLookbackPeriod,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub periods: ByLookbackPeriod<LazyPercentPerBlock<PartsPerMillionSigned64>>,
    pub cagr: ByDcaCagr<LazyPercentPerBlock<PartsPerMillionSigned64>>,
    pub sd_24h: Windows<StdDevPerBlock<M>>,
}
