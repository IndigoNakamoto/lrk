use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Dollars, PartsPerMillion64, StoredF32};
use vecdb::Exit;

use super::{Vecs, gini};
use crate::{distribution, internal::RatioDollars, market, mining};

impl Vecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        mining: &mining::Vecs,
        distribution: &distribution::Vecs,
        market: &market::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_lengths = indexer.safe_lengths();

        // Puell Multiple: daily_subsidy_usd / sma_365d_subsidy_usd
        self.puell_multiple
            .ppm
            .compute_binary::<Dollars, Dollars, RatioDollars<PartsPerMillion64>>(
                starting_lengths.height,
                &mining.rewards.subsidy.block.usd,
                &mining.rewards.subsidy.average._1y.usd.height,
                exit,
            )?;

        // Gini coefficient (UTXO distribution inequality)
        gini::compute(&mut self.gini, distribution, indexer, exit)?;

        // RHODL Ratio: 1d-1w realized cap / 1y-2y realized cap
        self.rhodl_ratio.ppm.height.compute_transform3(
            starting_lengths.height,
            &distribution
                .utxo_cohorts
                .age_range
                ._1d_to_1w
                .metrics
                .realized
                .cap
                .usd
                .height,
            &distribution
                .utxo_cohorts
                .age_range
                ._1y_to_18m
                .metrics
                .realized
                .cap
                .usd
                .height,
            &distribution
                .utxo_cohorts
                .age_range
                ._18m_to_2y
                .metrics
                .realized
                .cap
                .usd
                .height,
            |(i, young_cap, year1_cap, month18_cap, ..)| {
                let denominator = year1_cap + month18_cap;
                let ratio = f64::from(young_cap) / f64::from(denominator);
                (
                    i,
                    if ratio.is_finite() {
                        PartsPerMillion64::from(ratio)
                    } else {
                        PartsPerMillion64::default()
                    },
                )
            },
            exit,
        )?;

        let all_metrics = &distribution.utxo_cohorts.all.metrics;
        let supply_total_sats = &all_metrics.supply.total.sats.height;

        // Seller Exhaustion Constant: % supply_in_profit × 30d_volatility
        self.seller_exhaustion.height.compute_transform3(
            starting_lengths.height,
            &all_metrics.supply.in_profit.sats.height,
            &market.volatility._1m.height,
            supply_total_sats,
            |(i, profit_sats, volatility, total_sats, ..)| {
                let total = total_sats.as_u128() as f64;
                if total == 0.0 {
                    (i, StoredF32::from(0.0f32))
                } else {
                    let pct_in_profit = profit_sats.as_u128() as f64 / total;
                    (
                        i,
                        StoredF32::from((pct_in_profit * f64::from(volatility)) as f32),
                    )
                }
            },
            exit,
        )?;

        let exit = exit.clone();
        self.db.run_bg(move |db| {
            let _lock = exit.lock();
            db.compact_deferred_default()
        });
        Ok(())
    }
}
