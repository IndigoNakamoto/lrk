use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{blocks, distribution, mining, outputs, price, transactions};

impl Vecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        outputs: &outputs::Vecs,
        blocks: &blocks::Vecs,
        mining: &mining::Vecs,
        transactions: &transactions::Vecs,
        prices: &price::Vecs,
        distribution: &distribution::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_height = indexer.safe_lengths().height;

        // 1. Compute burned/unspendable supply
        self.burned
            .compute(indexer, outputs, mining, prices, exit)?;

        // 2. Compute velocity at height level
        self.velocity
            .compute(indexer, blocks, transactions, distribution, exit)?;

        // 3. market_cap_rate - realized_cap_rate per window
        let all_realized = &distribution.utxo_cohorts.all.metrics.realized;
        let mcr_arr = self.market_cap_delta.rate.as_array();
        let diff_arr = self.market_minus_realized_cap_growth_rate.0.as_mut_array();

        let rcr_rates = [
            &all_realized.cap.delta.rate._24h.ppm.height,
            &all_realized.cap.delta.rate._1w.ppm.height,
            &all_realized.cap.delta.rate._1m.ppm.height,
            &all_realized.cap.delta.rate._1y.ppm.height,
        ];

        for i in 0..4 {
            diff_arr[i].height.compute_subtract(
                starting_height,
                &mcr_arr[i].ppm.height,
                rcr_rates[i],
                exit,
            )?;
        }

        let exit = exit.clone();
        self.db.run_bg(move |db| {
            let _lock = exit.lock();
            db.compact_deferred_default()
        });

        Ok(())
    }
}
