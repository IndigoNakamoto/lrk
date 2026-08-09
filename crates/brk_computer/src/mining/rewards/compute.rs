use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{CheckedSub, Halving, Sats};
use vecdb::{Exit, ReadableVec, VecIndex};

use super::Vecs;
use crate::{blocks, indexes, price, transactions};

impl Vecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        lookback: &blocks::LookbackVecs,
        transactions: &transactions::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        // coinbase and fees are independent — parallelize
        let window_starts = lookback.window_starts();
        let (r_coinbase, r_fees) = rayon::join(
            || {
                self.coinbase.compute_from(
                    starting_height,
                    prices,
                    &indexer.vecs().transactions.first_tx_index,
                    |_, tx_index| {
                        let mut txout_cursor = indexer
                            .vecs()
                            .transactions
                            .first_txout_index
                            .reader()
                            .cursor();
                        let mut count_cursor = indexes.tx_index.output_count.cursor();

                        let ti = tx_index.to_usize();

                        txout_cursor.advance(ti - txout_cursor.position());
                        let first_txout_index = txout_cursor.next().unwrap().to_usize();

                        count_cursor.advance(ti - count_cursor.position());
                        let output_count: usize = count_cursor.next().unwrap().into();

                        indexer.vecs().outputs.value.fold_range_at(
                            first_txout_index,
                            first_txout_index + output_count,
                            Sats::ZERO,
                            |acc, v| acc + v,
                        )
                    },
                    exit,
                )
            },
            || {
                self.fees.compute_from_indexes(
                    starting_height,
                    &window_starts,
                    prices,
                    &indexer.vecs().transactions.first_tx_index,
                    &indexes.height.tx_index_count,
                    &transactions.fees.fee.tx_index,
                    exit,
                )
            },
        );
        r_coinbase?;
        r_fees?;

        self.subsidy.compute_from_pair(
            starting_height,
            prices,
            &self.coinbase.block.sats,
            &self.fees.block.sats,
            |height, coinbase, fees| {
                coinbase.checked_sub(fees).unwrap_or_else(|| {
                    panic!("coinbase {coinbase:?} < fees {fees:?} at {height:?}")
                })
            },
            exit,
        )?;

        self.output_volume.compute_subtract(
            starting_height,
            &transactions.volume.transfer_volume.block.sats,
            &self.fees.block.sats,
            exit,
        )?;

        self.unclaimed.compute_from(
            starting_height,
            prices,
            &self.subsidy.block.sats,
            |height, subsidy| {
                let halving = Halving::from(height);
                let expected = Sats::FIFTY_BTC / 2_usize.pow(halving.to_usize() as u32);
                expected.checked_sub(subsidy).unwrap()
            },
            exit,
        )?;

        Ok(())
    }
}
