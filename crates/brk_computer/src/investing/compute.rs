use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Bitcoin, Sats};
use vecdb::{AnyVec, Exit, ReadableOptionVec, ReadableVec, VecIndex};

use super::{DCA_AMOUNT, Vecs};
use crate::{indexes, price};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_height = indexer.safe_lengths().height;
        let h2d = &indexes.height.day1;
        let close = &prices.split.close.usd.day1;

        let start = self.sats_cumulative.len().min(starting_height.to_usize());
        let mut cumulative = start
            .checked_sub(1)
            .and_then(|height| self.sats_cumulative.collect_one_at(height))
            .unwrap_or_default();
        let mut last_day = start
            .checked_sub(1)
            .and_then(|height| h2d.collect_one_at(height));

        self.sats_cumulative.compute_transform(
            starting_height,
            h2d,
            |(height, day, _)| {
                if last_day != Some(day) {
                    cumulative += close
                        .collect_one_flat(day)
                        .map(sats_from_dca)
                        .unwrap_or_default();
                }
                last_day = Some(day);
                (height, cumulative)
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

fn sats_from_dca(price: brk_types::Dollars) -> Sats {
    if price == brk_types::Dollars::ZERO {
        Sats::ZERO
    } else {
        Sats::from(Bitcoin::from(DCA_AMOUNT / price))
    }
}
