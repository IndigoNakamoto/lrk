use brk_cohort::AgeRange;
use brk_traversable::Traversable;
use brk_types::{Cents, StoredF64};
use vecdb::{Rw, StorageMode};

use crate::internal::{
    FiatPerBlock, LazyPerBlock, PerBlock, PriceWithRatioPerBlock, SpotValuePerBlock,
};

#[derive(Clone, Copy, Traversable)]
pub struct Horizons<T> {
    pub _8y: T,
    pub _4y: T,
    pub _2y: T,
    pub _1y: T,
    pub _6m: T,
    pub _3m: T,
    pub _1m: T,
}

pub(crate) const HORIZON_NAMES: Horizons<&str> = Horizons {
    _8y: "8y",
    _4y: "4y",
    _2y: "2y",
    _1y: "1y",
    _6m: "6m",
    _3m: "3m",
    _1m: "1m",
};

pub(crate) const HORIZON_DAYS: Horizons<f64> = Horizons {
    _8y: 8.0 * 365.0,
    _4y: 4.0 * 365.0,
    _2y: 2.0 * 365.0,
    _1y: 365.0,
    _6m: 180.0,
    _3m: 90.0,
    _1m: 30.0,
};

impl<T> Horizons<T> {
    pub(crate) fn from_fn(mut create: impl FnMut(&'static str, f64) -> T) -> Self {
        let names = HORIZON_NAMES;
        let days = HORIZON_DAYS;
        Self {
            _8y: create(names._8y, days._8y),
            _4y: create(names._4y, days._4y),
            _2y: create(names._2y, days._2y),
            _1y: create(names._1y, days._1y),
            _6m: create(names._6m, days._6m),
            _3m: create(names._3m, days._3m),
            _1m: create(names._1m, days._1m),
        }
    }

    pub(crate) fn try_from_fn<E>(
        mut create: impl FnMut(&'static str, f64) -> Result<T, E>,
    ) -> Result<Self, E> {
        let names = HORIZON_NAMES;
        let days = HORIZON_DAYS;
        Ok(Self {
            _8y: create(names._8y, days._8y)?,
            _4y: create(names._4y, days._4y)?,
            _2y: create(names._2y, days._2y)?,
            _1y: create(names._1y, days._1y)?,
            _6m: create(names._6m, days._6m)?,
            _3m: create(names._3m, days._3m)?,
            _1m: create(names._1m, days._1m)?,
        })
    }

    pub(crate) fn as_array(&self) -> [&T; 7] {
        [
            &self._8y, &self._4y, &self._2y, &self._1y, &self._6m, &self._3m, &self._1m,
        ]
    }

    pub(crate) fn as_mut_array(&mut self) -> [&mut T; 7] {
        [
            &mut self._8y,
            &mut self._4y,
            &mut self._2y,
            &mut self._1y,
            &mut self._6m,
            &mut self._3m,
            &mut self._1m,
        ]
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.as_array().into_iter()
    }

    pub(crate) fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.as_mut_array().into_iter()
    }
}

#[derive(Traversable)]
pub struct HorizonVecs<M: StorageMode = Rw> {
    #[traversable(wrap = "supply/in_loss", rename = "share")]
    pub supply_in_loss_share: PerBlock<StoredF64, M>,
}

#[derive(Traversable)]
pub struct Split<T> {
    pub mobile: T,
    pub immobile: T,
}

#[derive(Traversable)]
pub struct CohortVecs<M: StorageMode = Rw> {
    pub spending_rate: PerBlock<StoredF64, M>,
    pub spending_exposure: PerBlock<StoredF64, M>,
    pub mobility: LazyPerBlock<StoredF64>,
    pub supply: Split<SpotValuePerBlock<M>>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub age_range: AgeRange<CohortVecs<M>>,
    pub supply: Split<SpotValuePerBlock<M>>,
    #[traversable(wrap = "supply/mobile/in_loss", rename = "share")]
    pub supply_in_loss_share: PerBlock<StoredF64, M>,
    pub horizon: Horizons<HorizonVecs<M>>,
    pub cap: FiatPerBlock<Cents, M>,
    pub price: PriceWithRatioPerBlock<M>,
}
