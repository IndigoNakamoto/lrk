use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, Sats, Version};
use vecdb::{Exit, Rw, StorageMode};

use crate::{
    distribution::{
        AllChainCache,
        metrics::{ImportConfig, SupplyCore, UnrealizedBasic},
    },
    internal::{LazyPercentPerBlock, PercentPerBlock, RatioSats},
};

/// Full relative metrics (sth/lth/all tier).
#[derive(Traversable)]
pub struct RelativeFull<M: StorageMode = Rw> {
    #[traversable(wrap = "supply/in_profit", rename = "share")]
    pub supply_in_profit_share: PercentPerBlock<PartsPerMillion32, M>,
    #[traversable(wrap = "supply/in_loss", rename = "share")]
    pub supply_in_loss_share: PercentPerBlock<PartsPerMillion32, M>,

    #[traversable(wrap = "unrealized/profit", rename = "to_mcap")]
    pub unrealized_profit_to_mcap: LazyPercentPerBlock<PartsPerMillion32>,
    #[traversable(wrap = "unrealized/loss", rename = "to_mcap")]
    pub unrealized_loss_to_mcap: LazyPercentPerBlock<PartsPerMillion32>,
}

impl RelativeFull {
    pub(crate) fn forced_import(
        cfg: &ImportConfig,
        unrealized: &UnrealizedBasic,
        all_chain: &AllChainCache,
    ) -> Result<Self> {
        let v1 = Version::ONE;
        let v2 = Version::new(2);
        let profit_name = cfg.name("unrealized_profit_to_mcap");
        let profit_source = all_chain.with_market_cap(
            &format!("{profit_name}_ppm_source"),
            v2,
            &unrealized.profit.cents.height,
            |_, profit, market_cap| Self::ratio_to_market_cap(profit, market_cap),
        );
        let loss_name = cfg.name("unrealized_loss_to_mcap");
        let loss_source = all_chain.with_market_cap(
            &format!("{loss_name}_ppm_source"),
            v2,
            &unrealized.loss.cents.height,
            |_, loss, market_cap| Self::ratio_to_market_cap(loss, market_cap),
        );

        Ok(Self {
            supply_in_profit_share: cfg.import("supply_in_profit_share", v1)?,
            supply_in_loss_share: cfg.import("supply_in_loss_share", v1)?,
            unrealized_profit_to_mcap: LazyPercentPerBlock::from_uncached_height_source(
                &profit_name,
                v2,
                profit_source,
                cfg.indexes,
            ),
            unrealized_loss_to_mcap: LazyPercentPerBlock::from_uncached_height_source(
                &loss_name,
                v2,
                loss_source,
                cfg.indexes,
            ),
        })
    }

    pub(crate) fn compute(
        &mut self,
        max_from: Height,
        supply: &SupplyCore,
        exit: &Exit,
    ) -> Result<()> {
        self.supply_in_profit_share
            .compute_binary::<Sats, Sats, RatioSats<PartsPerMillion32>>(
                max_from,
                &supply.in_profit.sats.height,
                &supply.total.sats.height,
                exit,
            )?;
        self.supply_in_loss_share
            .compute_binary::<Sats, Sats, RatioSats<PartsPerMillion32>>(
                max_from,
                &supply.in_loss.sats.height,
                &supply.total.sats.height,
                exit,
            )?;

        Ok(())
    }

    fn ratio_to_market_cap(
        value: brk_types::Cents,
        market_cap: brk_types::Cents,
    ) -> PartsPerMillion32 {
        let ratio = f64::from(value) / f64::from(market_cap);
        if ratio.is_finite() {
            PartsPerMillion32::from(ratio)
        } else {
            PartsPerMillion32::default()
        }
    }
}
