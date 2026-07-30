use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Bitcoin, Cents, PartsPerMillionSigned64, Sats};
use vecdb::{AnyVec, Exit, ReadableOptionVec, ReadableVec, VecIndex};

use super::vecs::DcaStack;
use super::{DCA_AMOUNT, Vecs};
use crate::{indexes, internal::{RatioDiffCents, SatsToCents}, price};

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

        let start = self
            .sats_cumulative
            .len()
            .min(starting_height.to_usize());
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

        for stack in self.period.dca_stack.iter_mut() {
            stack.compute_cents(prices, starting_height, exit)?;
        }
        for stack in self.class.dca_stack.iter_mut() {
            stack.compute_cents(prices, starting_height, exit)?;
        }

        for (returns, (cost_basis, _)) in self
            .period
            .dca_return
            .iter_mut()
            .zip(self.period.dca_cost_basis.iter_with_days())
        {
            returns.compute_binary::<Cents, Cents, RatioDiffCents<PartsPerMillionSigned64>>(
                starting_height,
                &prices.spot.cents.height,
                &cost_basis.cents.height,
                exit,
            )?;
        }

        for (returns, cost_basis) in self
            .class
            .dca_return
            .iter_mut()
            .zip(self.class.dca_cost_basis.iter())
        {
            returns.compute_binary::<Cents, Cents, RatioDiffCents<PartsPerMillionSigned64>>(
                starting_height,
                &prices.spot.cents.height,
                &cost_basis.cents.height,
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

impl DcaStack {
    fn compute_cents(
        &mut self,
        prices: &price::Vecs,
        starting_height: brk_types::Height,
        exit: &Exit,
    ) -> Result<()> {
        self.cents.compute_binary::<Sats, Cents, SatsToCents>(
            starting_height,
            &self.sats.height,
            &prices.spot.cents.height,
            exit,
        )
    }
}

fn sats_from_dca(price: brk_types::Dollars) -> Sats {
    if price == brk_types::Dollars::ZERO {
        Sats::ZERO
    } else {
        Sats::from(Bitcoin::from(DCA_AMOUNT / price))
    }
}
