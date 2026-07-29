use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{Database, Exit, Rw, StorageMode};

use crate::{
    indexes,
    internal::{
        LazyRollingAvgsAmountFromHeight, LazyRollingSumsAmountFromHeight, ValuePerBlockCumulative,
        WindowStartVec, Windows,
    },
    price,
};

#[derive(Deref, DerefMut, Traversable)]
pub struct ValuePerBlockCumulativeRolling<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub inner: ValuePerBlockCumulative<M>,
    pub sum: LazyRollingSumsAmountFromHeight,
    pub average: LazyRollingAvgsAmountFromHeight,
}

const VERSION: Version = Version::TWO;

impl ValuePerBlockCumulativeRolling {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let v = version + VERSION;

        let inner = ValuePerBlockCumulative::forced_import(db, name, v, indexes)?;
        let sum = LazyRollingSumsAmountFromHeight::new(
            &format!("{name}_sum"),
            v,
            &inner.cumulative.sats.height,
            &inner.cumulative.cents.height,
            cached_starts,
            indexes,
        );
        let average = LazyRollingAvgsAmountFromHeight::new(
            &format!("{name}_average"),
            v,
            &inner.cumulative.sats.height,
            &inner.cumulative.cents.height,
            cached_starts,
            indexes,
        );

        Ok(Self {
            inner,
            sum,
            average,
        })
    }

    pub(crate) fn compute_rest(
        &mut self,
        max_from: Height,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.inner.compute_cents(max_from, prices, exit)
    }
}
