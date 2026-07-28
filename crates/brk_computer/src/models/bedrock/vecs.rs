use brk_traversable::Traversable;
use brk_types::{Dollars, StoredF64};
use derive_more::{Deref, DerefMut};
use vecdb::{Database, Rw, StorageMode};

use super::urpd_metric::UrpdMetric;

pub(crate) const MODE_COUNT: usize = 10;

#[derive(Traversable)]
pub struct Percentiles<T> {
    pub pct95: T,
    pub pct98: T,
    pub pct99: T,
    pub pct99_5: T,
    pub pct99_9: T,
}

#[derive(Traversable)]
pub struct Levels<T> {
    pub pct10: T,
    pub pct20: T,
    pub pct30: T,
    pub pct40: T,
    pub pct50: T,
    pub pct60: T,
    pub pct70: T,
    pub pct80: T,
    pub pct90: T,
}

#[derive(Traversable)]
pub struct ModeVecs<M: StorageMode = Rw> {
    pub loss_threshold: Percentiles<UrpdMetric<StoredF64, M>>,
    pub floor: Percentiles<UrpdMetric<Dollars, M>>,
    pub level: Levels<UrpdMetric<Dollars, M>>,
}

#[derive(Traversable)]
pub struct Modes<T> {
    pub raw: T,
    pub cointime: T,
    pub coinflow: T,
    pub coinflow_8y: T,
    pub coinflow_4y: T,
    pub coinflow_2y: T,
    pub coinflow_1y: T,
    pub coinflow_6m: T,
    pub coinflow_3m: T,
    pub coinflow_1m: T,
}

impl<T> Modes<T> {
    pub(crate) fn try_from_fn<E>(
        mut create: impl FnMut(&'static str) -> Result<T, E>,
    ) -> Result<Self, E> {
        Ok(Self {
            raw: create("raw")?,
            cointime: create("cointime")?,
            coinflow: create("coinflow")?,
            coinflow_8y: create("coinflow_8y")?,
            coinflow_4y: create("coinflow_4y")?,
            coinflow_2y: create("coinflow_2y")?,
            coinflow_1y: create("coinflow_1y")?,
            coinflow_6m: create("coinflow_6m")?,
            coinflow_3m: create("coinflow_3m")?,
            coinflow_1m: create("coinflow_1m")?,
        })
    }

    pub(crate) fn as_mut_array(&mut self) -> [&mut T; MODE_COUNT] {
        [
            &mut self.raw,
            &mut self.cointime,
            &mut self.coinflow,
            &mut self.coinflow_8y,
            &mut self.coinflow_4y,
            &mut self.coinflow_2y,
            &mut self.coinflow_1y,
            &mut self.coinflow_6m,
            &mut self.coinflow_3m,
            &mut self.coinflow_1m,
        ]
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub modes: Modes<ModeVecs<M>>,
}
