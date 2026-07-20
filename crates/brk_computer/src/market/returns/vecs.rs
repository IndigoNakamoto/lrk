use brk_traversable::Traversable;
use brk_types::PartsPerMillionSigned64;
use vecdb::{Rw, StorageMode};

use crate::{
    internal::{PercentPerBlock, StdDevPerBlock, Windows},
    investing::ByDcaCagr,
    market::lookback::ByLookbackPeriod,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub periods: ByLookbackPeriod<PercentPerBlock<PartsPerMillionSigned64, M>>,
    pub cagr: ByDcaCagr<PercentPerBlock<PartsPerMillionSigned64, M>>,
    pub sd_24h: Windows<StdDevPerBlock<M>>,
}
