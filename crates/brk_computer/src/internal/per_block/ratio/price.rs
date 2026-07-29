use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Cents, Dollars, Height, PartsPerMillion64, SatsFract, StoredF32, Version};
use vecdb::{Database, EagerVec, Exit, PcoVec, ReadableVec, Rw, StorageMode};

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
        let ratio = RatioPerBlock::forced_import(db, name, version, indexes)?;
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
            |(i, close, price, ..)| {
                if price == Cents::ZERO {
                    (i, PartsPerMillion64::from(1.0))
                } else {
                    (
                        i,
                        PartsPerMillion64::from(f64::from(close) / f64::from(price)),
                    )
                }
            },
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
