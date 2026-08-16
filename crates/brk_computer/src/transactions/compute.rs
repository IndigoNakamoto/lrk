use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{blocks, indexes, inputs, price};

impl Vecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        inputs: &inputs::Vecs,
        indexes: &indexes::Vecs,
        blocks: &blocks::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let ((r1, r2), (r3, r4)) = rayon::join(
            || {
                rayon::join(
                    || self.count.compute(indexer, &blocks.lookback, exit),
                    || self.features.compute(indexer, exit),
                )
            },
            || {
                rayon::join(
                    || self.versions.compute(indexer, exit),
                    || self.size.compute(indexer, indexes, exit),
                )
            },
        );
        r1?;
        r2?;
        r3?;
        r4?;

        self.sigops.compute(indexer, indexes, exit)?;

        self.fees
            .compute(indexer, &inputs.value, indexes, &self.size, exit)?;

        self.patterns
            .compute(indexer, &inputs.value, indexes, exit)?;

        self.policy.compute(indexer, indexes, &self.fees, exit)?;

        self.hogex
            .compute(indexer, indexes, &self.fees, prices, exit)?;

        self.volume
            .compute(indexer, indexes, prices, &self.fees, exit)?;

        let exit = exit.clone();
        self.db.run_bg(move |db| {
            let _lock = exit.lock();
            db.compact_deferred_default()
        });
        Ok(())
    }
}
