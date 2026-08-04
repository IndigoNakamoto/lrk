use brk_traversable::Traversable;
use brk_types::{Dollars, Height, Version};
use vecdb::{LazyVec, ReadableCloneableVec};

use crate::internal::{FiatPerBlock, FiatType, LazyPreviousDeltaVec};

/// Per-block fiat data derived from stored cumulative cents.
#[derive(Clone, Traversable)]
pub struct LazyFiatBlock<C: FiatType> {
    pub usd: LazyVec<Height, Dollars, Height, C>,
    pub cents: LazyPreviousDeltaVec<Height, C>,
}

impl<C: FiatType> LazyFiatBlock<C> {
    pub(crate) fn from_cumulative(
        name: &str,
        version: Version,
        cumulative: &FiatPerBlock<C>,
    ) -> Self {
        let cents = LazyPreviousDeltaVec::new(
            &format!("{name}_cents"),
            version,
            cumulative.cents.height.read_only_boxed_clone(),
        );
        let usd =
            LazyVec::transformed::<C::ToDollars>(name, version, cents.read_only_boxed_clone());

        Self { usd, cents }
    }
}
