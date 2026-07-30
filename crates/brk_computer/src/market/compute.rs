use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use crate::{blocks, indexes, price};

use super::Vecs;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        indexes: &indexes::Vecs,
        blocks: &blocks::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        // Phase 1: Independent sub-modules in parallel
        let (r1, r2) = rayon::join(
            || self.ath.compute(indexer, prices, indexes, exit),
            || {
                rayon::join(
                    || self.range.compute(indexer, prices, blocks, exit),
                    || self.moving_average.compute(indexer, blocks, prices, exit),
                )
            },
        );
        r1?;
        r2.0?;
        r2.1?;

        // Phase 2: Stored volatility inputs derived from lazy 24h returns.
        self.returns.compute(indexer, blocks, exit)?;

        // Phase 3: Depends on returns, moving_average
        self.technical.compute(
            indexer,
            &self.returns,
            prices,
            blocks,
            &self.moving_average,
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
