use brk_traversable::Traversable;
use brk_types::{Bitcoin, Cents, Dollars, Height, PartsPerMillionSigned64, Sats};
use vecdb::{Database, EagerVec, PcoVec, Rw, StorageMode};

use super::{ByDcaCagr, ByDcaClass, ByDcaPeriod};
use crate::internal::{
    LazyPerBlock, LazyPercentPerBlock, LazyPreviousDeltaVec, PerBlock, PercentPerBlock, Price,
};

#[derive(Traversable)]
pub struct DcaStack<M: StorageMode = Rw> {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: LazyPerBlock<Sats>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: PerBlock<Cents, M>,
}

#[derive(Clone, Traversable)]
pub struct LumpSumStack {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: LazyPerBlock<Sats>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: LazyPerBlock<Cents>,
}

#[derive(Traversable)]
pub struct PeriodVecs<M: StorageMode = Rw> {
    pub dca_stack: ByDcaPeriod<DcaStack<M>>,
    pub dca_cost_basis: ByDcaPeriod<Price<LazyPerBlock<Cents>>>,
    pub dca_return: ByDcaPeriod<PercentPerBlock<PartsPerMillionSigned64, M>>,
    pub dca_cagr: ByDcaCagr<LazyPercentPerBlock<PartsPerMillionSigned64>>,
    pub lump_sum_stack: ByDcaPeriod<LumpSumStack>,
    pub lump_sum_return: ByDcaPeriod<LazyPercentPerBlock<PartsPerMillionSigned64>>,
}

#[derive(Traversable)]
pub struct ClassVecs<M: StorageMode = Rw> {
    pub dca_stack: ByDcaClass<DcaStack<M>>,
    pub dca_cost_basis: ByDcaClass<Price<LazyPerBlock<Cents>>>,
    pub dca_return: ByDcaClass<PercentPerBlock<PartsPerMillionSigned64, M>>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    pub sats_per_day: LazyPreviousDeltaVec<Height, Sats>,
    #[traversable(hidden)]
    pub sats_cumulative: M::Stored<EagerVec<PcoVec<Height, Sats>>>,
    pub period: PeriodVecs<M>,
    pub class: ClassVecs<M>,
}
