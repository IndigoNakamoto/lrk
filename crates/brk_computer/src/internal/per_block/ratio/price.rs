use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Cents, Dollars, Height, PartsPerMillion64, SatsFract, StoredF32, Version};
use vecdb::{Database, EagerVec, Exit, PcoVec, ReadableVec, Rw, StorageMode, unlikely};

use crate::internal::{LazyPerBlock, PerBlock, Price};
use crate::{indexes, price};

use super::RatioPerBlock;

#[derive(Traversable)]
pub struct PriceWithRatioPerBlock<M: StorageMode = Rw> {
    pub usd: LazyPerBlock<Dollars, Cents>,
    pub cents: PerBlock<Cents, M>,
    pub sats: LazyPerBlock<SatsFract, Dollars>,
    pub ppm: PerBlock<PartsPerMillion64, M>,
    pub ratio: LazyPerBlock<StoredF32, PartsPerMillion64>,
}

impl PriceWithRatioPerBlock {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let price = Price::forced_import(db, name, version, indexes)?;
        let ratio = RatioPerBlock::forced_import(db, name, version + Version::ONE, indexes)?;
        Ok(Self {
            usd: price.usd,
            cents: price.cents,
            sats: price.sats,
            ppm: ratio.ppm,
            ratio: ratio.ratio,
        })
    }

    /// Compute ratio from close price and this metric's price.
    pub(crate) fn compute_ratio(
        &mut self,
        starting_lengths: &Lengths,
        close_price: &impl ReadableVec<Height, Cents>,
        exit: &Exit,
    ) -> Result<()> {
        self.ppm.height.compute_transform2(
            starting_lengths.height,
            close_price,
            &self.cents.height,
            |(i, close, price, ..)| (i, price_ratio(close, price)),
            exit,
        )?;
        Ok(())
    }

    /// Compute price via closure (in cents), then compute ratio.
    pub(crate) fn compute_all<F>(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
        mut compute_price: F,
    ) -> Result<()>
    where
        F: FnMut(&mut EagerVec<PcoVec<Height, Cents>>) -> Result<()>,
    {
        compute_price(&mut self.cents.height)?;
        self.compute_ratio(starting_lengths, &prices.spot.cents.height, exit)
    }
}

fn price_ratio(close: Cents, price: Cents) -> PartsPerMillion64 {
    if unlikely(price == Cents::ZERO) {
        PartsPerMillion64::NAN
    } else {
        PartsPerMillion64::from(f64::from(close) / f64::from(price))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_price_has_no_ratio() {
        assert!(price_ratio(Cents::new(100), Cents::ZERO).is_nan());
        assert_eq!(
            price_ratio(Cents::new(100), Cents::new(50)),
            PartsPerMillion64::from(2.0),
        );
    }
}
