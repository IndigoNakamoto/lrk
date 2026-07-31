use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, PartsPerMillionSigned64, Sats, SatsSigned, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{CachedBoxedVec, Database, Rw, StorageMode};

use crate::{
    indexes,
    internal::{LazyRollingDeltasAmountFromHeight, SpotValuePerBlock, WindowStartVec, Windows},
};

#[derive(Deref, DerefMut, Traversable)]
pub struct SpotValuePerBlockWithDeltas<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub inner: SpotValuePerBlock<M>,
    pub delta: LazyRollingDeltasAmountFromHeight<Sats, SatsSigned, PartsPerMillionSigned64>,
}

impl SpotValuePerBlockWithDeltas {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let inner = SpotValuePerBlock::forced_import(db, name, version, indexes, spot_price)?;

        let delta = LazyRollingDeltasAmountFromHeight::new(
            &format!("{name}_delta"),
            version + Version::TWO,
            &inner.sats.height,
            cached_starts,
            indexes,
        );

        Ok(Self { inner, delta })
    }
}
