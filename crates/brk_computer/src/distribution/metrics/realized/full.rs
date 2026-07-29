use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{
    Bitcoin, Cents, CentsSats, CentsSigned, CentsSquaredSats, Dollars, Height, PartsPerMillion32,
    PartsPerMillionSigned64, StoredF64, Version,
};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, AnyVec, BytesVec, Exit, ReadableVec, Rw, StorageMode, WritableVec};

use crate::{
    distribution::state::{CohortState, CostBasisData, RealizedState, WithCapital},
    internal::{
        FiatPerBlockCumulativeWithSums, PercentPerBlock, PercentRollingWindows,
        PriceWithRatioPerBlock, RatioCents, RatioCents64, RatioCentsSignedCents,
        RatioCentsSignedDollars, RatioDollars, RollingWindows, RollingWindowsFrom1w,
        ValuePerBlockCumulativeRolling,
    },
    price,
};

use crate::distribution::metrics::ImportConfig;

use super::RealizedCore;

#[derive(Traversable)]
pub struct RealizedNetPnl<M: StorageMode = Rw> {
    #[traversable(wrap = "change_1m", rename = "to_rcap")]
    pub change_1m_to_rcap: PercentPerBlock<PartsPerMillionSigned64, M>,
    #[traversable(wrap = "change_1m", rename = "to_mcap")]
    pub change_1m_to_mcap: PercentPerBlock<PartsPerMillionSigned64, M>,
}

#[derive(Traversable)]
pub struct RealizedSopr<M: StorageMode = Rw> {
    #[traversable(rename = "ratio")]
    pub ratio_extended: RollingWindowsFrom1w<StoredF64, M>,
}

#[derive(Traversable)]
pub struct RealizedPeakRegret<M: StorageMode = Rw> {
    #[traversable(flatten)]
    pub value: FiatPerBlockCumulativeWithSums<Cents, M>,
}

#[derive(Traversable)]
pub struct RealizedCapitalized<M: StorageMode = Rw> {
    pub price: PriceWithRatioPerBlock<M>,
    #[traversable(hidden)]
    pub cap_raw: M::Stored<BytesVec<Height, CentsSquaredSats>>,
}

#[derive(Deref, DerefMut, Traversable)]
pub struct RealizedFull<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub core: RealizedCore<M>,

    pub gross_pnl: FiatPerBlockCumulativeWithSums<Cents, M>,
    pub sell_side_risk_ratio: PercentRollingWindows<PartsPerMillion32, M>,
    pub net_pnl: RealizedNetPnl<M>,
    pub sopr: RealizedSopr<M>,
    pub peak_regret: RealizedPeakRegret<M>,
    pub capitalized: RealizedCapitalized<M>,

    pub profit_to_loss_ratio: RollingWindows<StoredF64, M>,

    #[traversable(hidden)]
    pub cap_raw: M::Stored<BytesVec<Height, CentsSats>>,
    #[traversable(wrap = "cap", rename = "to_own_mcap")]
    pub cap_to_own_mcap: PercentPerBlock<PartsPerMillion32, M>,
}

impl RealizedFull {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v0 = Version::ZERO;
        let v1 = Version::ONE;

        let core = RealizedCore::forced_import(cfg)?;

        // Gross PnL
        let gross_pnl: FiatPerBlockCumulativeWithSums<Cents> =
            cfg.import("realized_gross_pnl", v1)?;
        let sell_side_risk_ratio = cfg.import("sell_side_risk_ratio", Version::new(2))?;

        // Net PnL
        let net_pnl = RealizedNetPnl {
            change_1m_to_rcap: cfg.import("net_pnl_change_1m_to_rcap", Version::new(5))?,
            change_1m_to_mcap: cfg.import("net_pnl_change_1m_to_mcap", Version::new(5))?,
        };

        // SOPR
        let sopr = RealizedSopr {
            ratio_extended: cfg.import("sopr", v1)?,
        };

        // Peak regret
        let peak_regret = RealizedPeakRegret {
            value: cfg.import("realized_peak_regret", Version::new(3))?,
        };

        // Capitalized
        let capitalized = RealizedCapitalized {
            price: cfg.import("capitalized_price", v0)?,
            cap_raw: cfg.import("capitalized_cap_raw", v0)?,
        };

        Ok(Self {
            core,
            gross_pnl,
            sell_side_risk_ratio,
            net_pnl,
            sopr,
            peak_regret,
            capitalized,
            profit_to_loss_ratio: cfg.import("realized_profit_to_loss_ratio", v1)?,
            cap_raw: cfg.import("cap_raw", v0)?,
            cap_to_own_mcap: cfg.import("realized_cap_to_own_mcap", v1)?,
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.capitalized
            .price
            .cents
            .height
            .len()
            .min(self.cap_raw.len())
            .min(self.capitalized.cap_raw.len())
            .min(self.peak_regret.value.cumulative.cents.height.len())
    }

    #[inline(always)]
    pub(crate) fn push_state(
        &mut self,
        state: &CohortState<RealizedState, CostBasisData<WithCapital>>,
    ) {
        self.core.push_state(state);
        self.capitalized
            .price
            .cents
            .height
            .push(state.realized.capitalized_price());
        self.cap_raw.push(state.realized.cap_raw());
        self.capitalized
            .cap_raw
            .push(state.realized.capitalized_cap_raw());
        self.peak_regret
            .value
            .push_block(state.realized.peak_regret());
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = self.core.collect_vecs_mut();
        vecs.push(&mut self.capitalized.price.cents.height);
        vecs.push(&mut self.cap_raw as &mut dyn AnyStoredVec);
        vecs.push(&mut self.capitalized.cap_raw as &mut dyn AnyStoredVec);
        vecs.push(self.peak_regret.value.stored_mut());
        vecs
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&RealizedCore],
        exit: &Exit,
    ) -> Result<()> {
        self.core
            .compute_from_stateful(starting_lengths, others, exit)
    }

    #[inline(always)]
    pub(crate) fn push_accum(&mut self, accum: &RealizedFullAccum) -> Cents {
        self.cap_raw.push(accum.cap_raw);
        self.capitalized.cap_raw.push(accum.capitalized_cap_raw);

        let capitalized_price = {
            let cap = accum.cap_raw.as_u128();
            if cap == 0 {
                Cents::ZERO
            } else {
                Cents::new((accum.capitalized_cap_raw / cap) as u64)
            }
        };
        self.capitalized.price.cents.height.push(capitalized_price);

        self.peak_regret.value.push_block(accum.peak_regret());

        capitalized_price
    }

    pub(crate) fn compute_rest_part1(
        &mut self,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.core.compute_rest_part1(starting_lengths, exit)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute_rest_part2(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        height_to_supply: &impl ReadableVec<Height, Bitcoin>,
        height_to_market_cap: &impl ReadableVec<Height, Dollars>,
        activity_transfer_volume: &ValuePerBlockCumulativeRolling,
        exit: &Exit,
    ) -> Result<()> {
        self.core.compute_rest_part2(
            prices,
            starting_lengths,
            height_to_supply,
            &activity_transfer_volume.sum._24h.cents.height,
            exit,
        )?;

        // SOPR ratios from lazy rolling sums (1w, 1m, 1y)
        for ((sopr, vc), vd) in self
            .sopr
            .ratio_extended
            .as_mut_array()
            .into_iter()
            .zip(activity_transfer_volume.sum.0.as_array()[1..].iter())
            .zip(self.core.sopr.value_destroyed.sum.as_array()[1..].iter())
        {
            sopr.compute_binary::<Cents, Cents, RatioCents64>(
                starting_lengths.height,
                &vc.cents.height,
                &vd.height,
                exit,
            )?;
        }

        // Gross PnL
        self.gross_pnl.compute_from_cumulative_pair(
            starting_lengths.height,
            &self.core.minimal.profit.cumulative.cents.height,
            &self.core.minimal.loss.cumulative.cents.height,
            |_, profit, loss| profit + loss,
            exit,
        )?;

        // Net PnL 1m change relative to rcap and mcap
        self.net_pnl
            .change_1m_to_rcap
            .compute_binary::<CentsSigned, Cents, RatioCentsSignedCents<PartsPerMillionSigned64>>(
                starting_lengths.height,
                &self.core.net_pnl.delta.absolute._1m.cents.height,
                &self.core.minimal.cap.cents.height,
                exit,
            )?;
        self.net_pnl
            .change_1m_to_mcap
            .compute_binary::<
                CentsSigned,
                Dollars,
                RatioCentsSignedDollars<PartsPerMillionSigned64>,
            >(
                starting_lengths.height,
                &self.core.net_pnl.delta.absolute._1m.cents.height,
                height_to_market_cap,
                exit,
            )?;

        // Capitalized price ratio
        self.capitalized
            .price
            .compute_ratio(starting_lengths, &prices.spot.cents.height, exit)?;

        // Sell-side risk ratios
        for (ssrr, rv) in self
            .sell_side_risk_ratio
            .as_mut_array()
            .into_iter()
            .zip(self.gross_pnl.sum.as_array())
        {
            ssrr.compute_binary::<Cents, Cents, RatioCents<PartsPerMillion32>>(
                starting_lengths.height,
                &rv.cents.height,
                &self.core.minimal.cap.cents.height,
                exit,
            )?;
        }

        // Realized cap relative to own market cap
        self.cap_to_own_mcap
            .compute_binary::<Dollars, Dollars, RatioDollars<PartsPerMillion32>>(
                starting_lengths.height,
                &self.core.minimal.cap.usd.height,
                height_to_market_cap,
                exit,
            )?;

        // Realized profit to loss ratios
        for ((ratio, profit), loss) in self
            .profit_to_loss_ratio
            .as_mut_array()
            .into_iter()
            .zip(self.core.minimal.profit.sum.as_array())
            .zip(self.core.minimal.loss.sum.as_array())
        {
            ratio.compute_binary::<Cents, Cents, RatioCents64>(
                starting_lengths.height,
                &profit.cents.height,
                &loss.cents.height,
                exit,
            )?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct RealizedFullAccum {
    pub(crate) cap_raw: CentsSats,
    pub(crate) capitalized_cap_raw: CentsSquaredSats,
    peak_regret: CentsSats,
}

impl RealizedFullAccum {
    pub(crate) fn add(&mut self, state: &RealizedState) {
        self.cap_raw += state.cap_raw();
        self.capitalized_cap_raw += state.capitalized_cap_raw();
        self.peak_regret += CentsSats::new(state.peak_regret_raw());
    }

    pub(crate) fn peak_regret(&self) -> Cents {
        self.peak_regret.to_cents()
    }
}
