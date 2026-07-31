use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Bitcoin, Cents, Dollars, Height, Sats, Version};
use vecdb::{BinaryTransform, CachedBoxedVec, Database, ReadableCloneableVec, Rw, StorageMode};

use crate::{
    indexes,
    internal::{
        CentsUnsignedToDollars, Identity, LazyIndexedVec, LazyPerBlock, PerBlock, SatsToBitcoin,
        SatsToCents,
    },
};

/// A point-in-time value whose fiat amount is derived from sats and same-height spot price.
#[derive(Traversable)]
pub struct SpotValuePerBlock<M: StorageMode = Rw> {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: PerBlock<Sats, M>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: LazyPerBlock<Cents>,
}

impl SpotValuePerBlock {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let sats = PerBlock::forced_import(db, &format!("{name}_sats"), version, indexes)?;

        let btc = LazyPerBlock::from_computed::<SatsToBitcoin>(
            name,
            version,
            sats.height.read_only_boxed_clone(),
            &sats,
        );

        let cents_source = LazyIndexedVec::new(
            &format!("{name}_cents_source"),
            version,
            sats.height.read_only_boxed_clone(),
            spot_price.clone(),
            |_, sats, spot| SatsToCents::apply(sats, spot),
        );
        let cents = LazyPerBlock::from_height_source::<Identity<Cents>, _>(
            &format!("{name}_cents"),
            version,
            cents_source,
            indexes,
        );

        let usd = LazyPerBlock::from_lazy::<CentsUnsignedToDollars, Cents>(
            &format!("{name}_usd"),
            version,
            &cents,
        );

        Ok(Self {
            btc,
            sats,
            usd,
            cents,
        })
    }
}
