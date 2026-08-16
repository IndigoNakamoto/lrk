use brk_traversable::Traversable;
use brk_types::{Bitcoin, Cents, Dollars, Height, Sats, Version};
use vecdb::{BinaryTransform, CachedBoxedVec, ReadableCloneableVec, ReadableVec, TypedVec};

use crate::{
    indexes,
    internal::{
        CentsUnsignedToDollars, Identity, LazyIndexedVec, LazyPerBlock, SatsToBitcoin, SatsToCents,
    },
};

/// Fully lazy point-in-time value backed by one sats source.
#[derive(Clone, Traversable)]
pub struct LazySpotValuePerBlock {
    pub btc: LazyPerBlock<Bitcoin, Sats>,
    pub sats: LazyPerBlock<Sats>,
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: LazyPerBlock<Cents>,
}

impl LazySpotValuePerBlock {
    pub(crate) fn identity(name: &str, version: Version, source: &Self) -> Self {
        let sats = LazyPerBlock::from_lazy::<Identity<Sats>, Sats>(
            &format!("{name}_sats"),
            version,
            &source.sats,
        );
        let btc = LazyPerBlock::from_lazy::<SatsToBitcoin, Sats>(name, version, &source.sats);
        let cents = LazyPerBlock::from_lazy::<Identity<Cents>, Cents>(
            &format!("{name}_cents"),
            version,
            &source.cents,
        );
        let usd = LazyPerBlock::from_lazy::<CentsUnsignedToDollars, Cents>(
            &format!("{name}_usd"),
            version,
            &source.cents,
        );

        Self {
            btc,
            sats,
            usd,
            cents,
        }
    }

    pub(crate) fn from_sats_source<V>(
        name: &str,
        version: Version,
        source: V,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Self
    where
        V: TypedVec<I = Height, T = Sats> + ReadableVec<Height, Sats> + Clone + 'static,
    {
        let sats = LazyPerBlock::from_uncached_height_source::<Identity<Sats>, _>(
            &format!("{name}_sats"),
            version,
            source,
            indexes,
        );
        let btc = LazyPerBlock::from_lazy::<SatsToBitcoin, Sats>(name, version, &sats);
        let cents_source = LazyIndexedVec::new(
            &format!("{name}_cents_source"),
            version,
            sats.height.read_only_boxed_clone(),
            spot_price.clone(),
            |_, sats, spot| SatsToCents::apply(sats, spot),
        );
        let cents = LazyPerBlock::from_uncached_height_source::<Identity<Cents>, _>(
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

        Self {
            btc,
            sats,
            usd,
            cents,
        }
    }
}
