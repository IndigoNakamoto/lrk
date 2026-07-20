use brk_traversable::Traversable;
use brk_types::{Cents, Height, PartsPerMillionSigned64, Sats};
use vecdb::{Database, EagerVec, PcoVec, Rw, StorageMode};

use super::{ByDcaCagr, ByDcaClass, ByDcaPeriod};
use crate::internal::{PerBlock, PercentPerBlock, Price, ValuePerBlock};

#[derive(Traversable)]
pub struct PeriodVecs<M: StorageMode = Rw> {
    pub dca_stack: ByDcaPeriod<ValuePerBlock<M>>,
    pub dca_cost_basis: ByDcaPeriod<Price<PerBlock<Cents, M>>>,
    pub dca_return: ByDcaPeriod<PercentPerBlock<PartsPerMillionSigned64, M>>,
    pub dca_cagr: ByDcaCagr<PercentPerBlock<PartsPerMillionSigned64, M>>,
    pub lump_sum_stack: ByDcaPeriod<ValuePerBlock<M>>,
    pub lump_sum_return: ByDcaPeriod<PercentPerBlock<PartsPerMillionSigned64, M>>,
}

#[derive(Traversable)]
pub struct ClassVecs<M: StorageMode = Rw> {
    pub dca_stack: ByDcaClass<ValuePerBlock<M>>,
    pub dca_cost_basis: ByDcaClass<Price<PerBlock<Cents, M>>>,
    pub dca_return: ByDcaClass<PercentPerBlock<PartsPerMillionSigned64, M>>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    pub sats_per_day: M::Stored<EagerVec<PcoVec<Height, Sats>>>,
    pub period: PeriodVecs<M>,
    pub class: ClassVecs<M>,
}
