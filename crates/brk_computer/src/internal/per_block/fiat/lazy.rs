use brk_traversable::Traversable;
use brk_types::{Dollars, Version};

use crate::internal::{FiatType, Identity, LazyPerBlock, NumericValue};

/// Lazy fiat: both cents and usd are lazy views of a stored source.
/// Zero extra stored vecs.
#[derive(Clone, Traversable)]
pub struct LazyFiatPerBlock<C: FiatType> {
    pub usd: LazyPerBlock<Dollars, C>,
    pub cents: LazyPerBlock<C, C>,
}

impl<C: FiatType> LazyFiatPerBlock<C> {
    pub(crate) fn from_lazy(name: &str, version: Version, source: &LazyPerBlock<C>) -> Self
    where
        C: NumericValue,
    {
        let cents =
            LazyPerBlock::from_lazy::<Identity<C>, C>(&format!("{name}_cents"), version, source);
        let usd = LazyPerBlock::from_lazy::<C::ToDollars, C>(name, version, source);
        Self { usd, cents }
    }
}
