use brk_traversable::Traversable;
use brk_types::{Cents, Dollars, OHLCCents, OHLCDollars, OHLCSats, Sats};
use vecdb::{Rw, StorageMode};

use crate::internal::{CachedPerBlock, LazyIndexes, LazyPerBlock, Resolutions};

use super::ohlcs::{LazyOhlcCentsVecs, LazyOhlcVecs};

#[derive(Clone, Traversable)]
pub struct SplitByUnit {
    pub open: SplitIndexesByUnit,
    pub high: SplitIndexesByUnit,
    pub low: SplitIndexesByUnit,
    pub close: SplitCloseByUnit,
}

#[derive(Clone, Traversable)]
pub struct SplitIndexesByUnit {
    pub usd: LazyIndexes<Dollars, Cents>,
    pub cents: LazyIndexes<Cents, OHLCCents>,
    pub sats: LazyIndexes<Sats, Cents>,
}

#[derive(Clone, Traversable)]
pub struct SplitCloseByUnit {
    pub usd: Resolutions<Dollars>,
    pub cents: Resolutions<Cents>,
    pub sats: Resolutions<Sats>,
}

#[derive(Clone, Traversable)]
pub struct OhlcByUnit {
    pub usd: LazyOhlcVecs<OHLCDollars, OHLCCents>,
    pub cents: LazyOhlcCentsVecs,
    pub sats: LazyOhlcVecs<OHLCSats, OHLCCents>,
}

#[derive(Traversable)]
pub struct PriceByUnit<M: StorageMode = Rw> {
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: CachedPerBlock<Cents, M>,
    pub sats: LazyPerBlock<Sats, Cents>,
}
