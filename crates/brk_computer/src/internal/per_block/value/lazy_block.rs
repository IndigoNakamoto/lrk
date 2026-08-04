use brk_traversable::Traversable;
use brk_types::{Bitcoin, Cents, Dollars, Height, Sats, Version};
use vecdb::{LazyVecFrom1, ReadableCloneableVec};

use crate::internal::{CentsUnsignedToDollars, LazyPreviousDeltaVec, SatsToBitcoin, ValuePerBlock};

/// Per-block amount data derived from stored cumulative sats and cents.
#[derive(Clone, Traversable)]
pub struct LazyValueBlock {
    pub btc: LazyVecFrom1<Height, Bitcoin, Height, Sats>,
    pub sats: LazyPreviousDeltaVec<Height, Sats>,
    pub usd: LazyVecFrom1<Height, Dollars, Height, Cents>,
    pub cents: LazyPreviousDeltaVec<Height, Cents>,
}

impl LazyValueBlock {
    pub(crate) fn from_cumulative(
        name: &str,
        version: Version,
        cumulative: &ValuePerBlock,
    ) -> Self {
        Self::from_cumulative_sources(
            name,
            version,
            &cumulative.sats.height,
            &cumulative.cents.height,
        )
    }

    pub(crate) fn from_cumulative_sources(
        name: &str,
        version: Version,
        cumulative_sats: &(impl ReadableCloneableVec<Height, Sats> + 'static),
        cumulative_cents: &(impl ReadableCloneableVec<Height, Cents> + 'static),
    ) -> Self {
        let sats = LazyPreviousDeltaVec::new(
            &format!("{name}_sats"),
            version,
            cumulative_sats.read_only_boxed_clone(),
        );
        let btc =
            LazyVecFrom1::transformed::<SatsToBitcoin>(name, version, sats.read_only_boxed_clone());
        let cents = LazyPreviousDeltaVec::new(
            &format!("{name}_cents"),
            version,
            cumulative_cents.read_only_boxed_clone(),
        );
        let usd = LazyVecFrom1::transformed::<CentsUnsignedToDollars>(
            &format!("{name}_usd"),
            version,
            cents.read_only_boxed_clone(),
        );

        Self {
            btc,
            sats,
            usd,
            cents,
        }
    }
}
