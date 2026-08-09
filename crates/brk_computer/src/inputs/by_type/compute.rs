use brk_error::{OptionData, Result};
use brk_indexer::Indexer;
use vecdb::{AnyVec, Exit, ReadableVec, VecIndex};

use super::Vecs;
use crate::internal::{CoinbasePolicy, walk_blocks};

const WRITE_INTERVAL: usize = 10_000;

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        let dep_version = indexer.vecs().inputs.output_type.version()
            + indexer.vecs().transactions.first_tx_index.version()
            + indexer.vecs().transactions.first_txin_index.version()
            + indexer.vecs().transactions.txid.version();

        self.input_count
            .validate_and_truncate(dep_version, starting_lengths.height)?;
        self.tx_count
            .validate_and_truncate(dep_version, starting_lengths.height)?;

        let skip = self
            .input_count
            .min_stateful_len()
            .min(self.tx_count.min_stateful_len());

        let first_tx_index = &indexer.vecs().transactions.first_tx_index;
        let end = first_tx_index.len();
        if skip < end {
            self.input_count.truncate_if_needed_at(skip)?;
            self.tx_count.truncate_if_needed_at(skip)?;

            let fi_batch = first_tx_index.collect_range_at(skip, end);
            let txid_len = indexer.vecs().transactions.txid.len();
            let total_txin_len = indexer.vecs().inputs.output_type.len();

            let mut itype_cursor = indexer.vecs().inputs.output_type.cursor();
            let mut fi_in_cursor = indexer.vecs().transactions.first_txin_index.cursor();
            let mut height = skip;

            walk_blocks(
                &fi_batch,
                txid_len,
                CoinbasePolicy::Skip,
                |tx_pos, per_tx| {
                    let fi_in = fi_in_cursor.get(tx_pos).data()?.to_usize();
                    let next_fi_in = if tx_pos + 1 < txid_len {
                        fi_in_cursor.get(tx_pos + 1).data()?.to_usize()
                    } else {
                        total_txin_len
                    };

                    itype_cursor.advance(fi_in - itype_cursor.position());
                    itype_cursor.for_each(next_fi_in - fi_in, |otype| {
                        per_tx[otype as usize] += 1;
                    });
                    Ok(())
                },
                |agg| {
                    self.input_count.push_block(&agg.entries_per_type);
                    self.tx_count.push_block(&agg.txs_per_type);

                    height += 1;
                    if height.is_multiple_of(WRITE_INTERVAL) {
                        let _lock = exit.lock();
                        self.input_count.write()?;
                        self.tx_count.write()?;
                    }
                    Ok(())
                },
            )?;

            {
                let _lock = exit.lock();
                self.input_count.write()?;
                self.tx_count.write()?;
            }
        }

        Ok(())
    }
}
