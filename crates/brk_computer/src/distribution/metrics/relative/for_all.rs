use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::Height;
use derive_more::{Deref, DerefMut};
use vecdb::{Exit, Rw, StorageMode};

use crate::distribution::{
    AllChainCache,
    metrics::{ImportConfig, RealizedFull, SupplyCore, UnrealizedFull},
};

use super::{RelativeExtendedOwnPnl, RelativeFull, RelativeInvestedCapital};

/// Relative metrics for the "all" cohort (base + own_pnl, NO rel_to_all).
#[derive(Deref, DerefMut, Traversable)]
pub struct RelativeForAll<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub base: RelativeFull<M>,
    #[traversable(flatten)]
    pub extended_own_pnl: RelativeExtendedOwnPnl<M>,
    #[traversable(flatten)]
    pub invested_capital: RelativeInvestedCapital<M>,
}

impl RelativeForAll {
    pub(crate) fn forced_import(
        cfg: &ImportConfig,
        unrealized: &UnrealizedFull,
        all_chain: &AllChainCache,
    ) -> Result<Self> {
        Ok(Self {
            base: RelativeFull::forced_import(cfg, &unrealized.inner.basic, all_chain)?,
            extended_own_pnl: RelativeExtendedOwnPnl::forced_import(cfg)?,
            invested_capital: RelativeInvestedCapital::forced_import(cfg)?,
        })
    }

    pub(crate) fn compute(
        &mut self,
        max_from: Height,
        supply: &SupplyCore,
        unrealized: &UnrealizedFull,
        realized: &RealizedFull,
        exit: &Exit,
    ) -> Result<()> {
        self.base.compute(max_from, supply, exit)?;
        self.extended_own_pnl.compute(
            max_from,
            &unrealized.inner,
            &unrealized.gross_pnl.usd.height,
            exit,
        )?;
        self.invested_capital
            .compute(max_from, unrealized, realized, exit)?;
        Ok(())
    }
}
