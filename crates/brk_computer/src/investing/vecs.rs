use brk_traversable::Traversable;
use brk_types::{Bitcoin, Cents, Dollars, Height, PartsPerMillionSigned64, Sats};

use super::cached_dca_sats::CachedDcaSats;
use super::{ByDcaCagr, ByDcaClass, ByDcaPeriod};
use crate::internal::{LazyPerBlock, LazyPercentPerBlock, LazyPreviousDeltaVec, Price};

#[derive(Clone, Traversable)]
pub struct DcaStack {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: LazyPerBlock<Sats>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: LazyPerBlock<Cents>,
}

#[derive(Clone, Traversable)]
pub struct LumpSumStack {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: LazyPerBlock<Sats>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: LazyPerBlock<Cents>,
}

#[derive(Clone, Traversable)]
pub struct PeriodVecs {
    pub dca_stack: ByDcaPeriod<DcaStack>,
    pub dca_cost_basis: ByDcaPeriod<Price<LazyPerBlock<Cents>>>,
    pub dca_return: ByDcaPeriod<LazyPercentPerBlock<PartsPerMillionSigned64>>,
    pub dca_cagr: ByDcaCagr<LazyPercentPerBlock<PartsPerMillionSigned64>>,
    pub lump_sum_stack: ByDcaPeriod<LumpSumStack>,
    pub lump_sum_return: ByDcaPeriod<LazyPercentPerBlock<PartsPerMillionSigned64>>,
}

#[derive(Clone, Traversable)]
pub struct ClassVecs {
    pub dca_stack: ByDcaClass<DcaStack>,
    pub dca_cost_basis: ByDcaClass<Price<LazyPerBlock<Cents>>>,
    pub dca_return: ByDcaClass<LazyPercentPerBlock<PartsPerMillionSigned64>>,
}

#[derive(Clone, Traversable)]
pub struct Vecs {
    #[traversable(skip)]
    pub(super) cached_dca_sats: CachedDcaSats,
    pub sats_per_day: LazyPreviousDeltaVec<Height, Sats>,
    pub period: PeriodVecs,
    pub class: ClassVecs,
}

impl Vecs {
    pub(crate) fn invalidate_cache(&self) {
        self.cached_dca_sats.invalidate();
    }
}
