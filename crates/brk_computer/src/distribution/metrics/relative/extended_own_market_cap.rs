use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Dollars, Height, PartsPerMillion32, PartsPerMillionSigned32, Version};
use vecdb::{Exit, ReadableVec, Rw, StorageMode};

use crate::internal::{LazyPercentPerBlock, PercentPerBlock, RatioDollars};

use crate::distribution::metrics::{ImportConfig, UnrealizedCore};

/// Extended relative metrics for own market cap (extended && rel_to_all).
#[derive(Traversable)]
pub struct RelativeExtendedOwnMarketCap<M: StorageMode = Rw> {
    #[traversable(wrap = "unrealized/net_pnl", rename = "to_own_mcap")]
    pub net_unrealized_pnl_to_own_mcap: LazyPercentPerBlock<PartsPerMillionSigned32>,
    #[traversable(wrap = "unrealized/profit", rename = "to_own_mcap")]
    pub unrealized_profit_to_own_mcap: PercentPerBlock<PartsPerMillion32, M>,
    #[traversable(wrap = "unrealized/loss", rename = "to_own_mcap")]
    pub unrealized_loss_to_own_mcap: PercentPerBlock<PartsPerMillion32, M>,
}

impl RelativeExtendedOwnMarketCap {
    pub(crate) fn forced_import(cfg: &ImportConfig, unrealized: &UnrealizedCore) -> Result<Self> {
        let v2 = Version::new(2);

        Ok(Self {
            net_unrealized_pnl_to_own_mcap: LazyPercentPerBlock::from_height_source(
                &cfg.name("net_unrealized_pnl_to_own_mcap"),
                cfg.version + Version::new(4),
                unrealized.nupl.ppm.height.clone(),
                cfg.indexes,
            ),
            unrealized_profit_to_own_mcap: cfg.import("unrealized_profit_to_own_mcap", v2)?,
            unrealized_loss_to_own_mcap: cfg
                .import("unrealized_loss_to_own_mcap", Version::new(3))?,
        })
    }

    pub(crate) fn compute(
        &mut self,
        max_from: Height,
        unrealized: &UnrealizedCore,
        own_market_cap: &impl ReadableVec<Height, Dollars>,
        exit: &Exit,
    ) -> Result<()> {
        self.unrealized_profit_to_own_mcap
            .compute_binary::<Dollars, Dollars, RatioDollars<PartsPerMillion32>>(
                max_from,
                &unrealized.profit.usd.height,
                own_market_cap,
                exit,
            )?;
        self.unrealized_loss_to_own_mcap
            .compute_binary::<Dollars, Dollars, RatioDollars<PartsPerMillion32>>(
                max_from,
                &unrealized.loss.usd.height,
                own_market_cap,
                exit,
            )?;
        Ok(())
    }
}
