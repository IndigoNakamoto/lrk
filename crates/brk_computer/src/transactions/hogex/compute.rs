use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{StoredU32, TxIndex};
use vecdb::{AnyVec, Exit, ReadableVec, VecIndex};

use super::Vecs;
use crate::{indexes, price, transactions::fees};

const WRITE_INTERVAL: usize = 10_000;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        fees: &fees::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let txs = &indexer.vecs().transactions;

        let dep_version = txs.first_tx_index.version() + txs.is_hog_ex.version();
        self.tx_count
            .validate_and_truncate(dep_version, starting_height)?;

        let skip = self.tx_count.cumulative.height.len();
        let end = txs.first_tx_index.len();
        if skip < end {
            self.tx_count.truncate_if_needed_at(skip)?;

            let first_tx_indexes = txs.first_tx_index.collect_range_at(skip, end);
            let txid_len = txs.txid.len();
            let mut hogex_cursor = txs.is_hog_ex.cursor();
            if skip < end {
                hogex_cursor.advance(first_tx_indexes[0].to_usize());
            }

            for (local_idx, first_tx) in first_tx_indexes.iter().enumerate() {
                let first = first_tx.to_usize();
                let next = first_tx_indexes
                    .get(local_idx + 1)
                    .copied()
                    .unwrap_or_else(|| TxIndex::from(txid_len))
                    .to_usize();
                hogex_cursor.advance(first.saturating_sub(hogex_cursor.position()));
                let hogex_count = hogex_cursor.fold(next - first, 0u32, |acc, flag| {
                    if flag.is_true() { acc + 1 } else { acc }
                });
                self.tx_count.push_block(StoredU32::from(hogex_count));

                if (skip + local_idx + 1).is_multiple_of(WRITE_INTERVAL) {
                    let _lock = exit.lock();
                    self.tx_count.write()?;
                }
            }

            let _lock = exit.lock();
            self.tx_count.write()?;
        }

        self.raw_input_volume.compute_filtered_from_indexes(
            starting_height,
            prices,
            &txs.first_tx_index,
            &indexes.height.tx_index_count,
            &fees.input_value,
            |sats| !sats.is_max(),
            exit,
        )?;

        Ok(())
    }
}
