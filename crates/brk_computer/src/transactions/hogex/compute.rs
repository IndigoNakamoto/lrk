use std::time::{Duration, Instant};

use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, StoredU32, TxIndex};
use tracing::info;
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecIndex, WritableVec};

use super::Vecs;
use crate::{indexes, price, transactions::fees};

const PROGRESS_BLOCKS: usize = 50_000;
const PROGRESS_INTERVAL: Duration = Duration::from_secs(30);

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

        let dep_version = indexer.vecs.transactions.first_tx_index.version()
            + indexer.vecs.transactions.is_hog_ex.version();
        self.tx_count
            .block
            .validate_computed_version_or_reset(dep_version)?;

        self.tx_count.compute(starting_height, exit, |height_vec| {
            let target_len = indexer.vecs.transactions.first_tx_index.len();
            if target_len == 0 {
                return Ok(());
            }
            let target_height = Height::from(target_len - 1);

            let current_len = height_vec.len();
            let starting_height =
                Height::from(current_len.min(starting_height.to_usize()));
            if starting_height > target_height {
                return Ok(());
            }

            let first_tx_indexes: Vec<TxIndex> = indexer.vecs.transactions.first_tx_index.collect_range_at(
                starting_height.to_usize(),
                target_height.to_usize() + 2.min(indexer.vecs.transactions.first_tx_index.len()),
            );

            height_vec.truncate_if_needed(starting_height)?;

            let start = starting_height.to_usize();
            let end = target_height.to_usize();
            info!("Computing hogex_tx_count for heights {start}..={end} ({} blocks)...", end - start + 1);

            // Sequential cursors avoid per-tx page decompression (see pools/mod.rs).
            let mut hogex_cursor = indexer.vecs.transactions.is_hog_ex.cursor();
            if start < end {
                hogex_cursor.advance(first_tx_indexes[0].to_usize());
            }

            let mut last_log = Instant::now();

            for h in start..=end {
                let local_idx = h - start;
                let first_tx = first_tx_indexes[local_idx].to_usize();
                let next_tx = first_tx_indexes
                    .get(local_idx + 1)
                    .copied()
                    .unwrap_or_else(|| TxIndex::from(indexer.vecs.transactions.txid.len()))
                    .to_usize();

                hogex_cursor.advance(first_tx.saturating_sub(hogex_cursor.position()));
                let n_txs = next_tx - first_tx;
                let hogex_count = hogex_cursor.fold(n_txs, 0u32, |acc, flag| {
                    if flag.is_true() { acc + 1 } else { acc }
                });

                height_vec.push(StoredU32::from(hogex_count));

                if h == start
                    || h == end
                    || (h - start) % PROGRESS_BLOCKS == 0
                    || last_log.elapsed() >= PROGRESS_INTERVAL
                {
                    let done = h - start + 1;
                    let total = end - start + 1;
                    info!(
                        "hogex_tx_count: height {h}/{end} ({:.1}%)",
                        done as f64 / total as f64 * 100.0
                    );
                    last_log = Instant::now();
                }
            }

            height_vec.write()?;
            info!("hogex_tx_count complete");
            Ok(())
        })?;

        self.raw_input_volume
            .compute(starting_height, prices, exit, |sats_vec| {
                Ok(sats_vec.compute_filtered_sum_from_indexes(
                    starting_height,
                    &indexer.vecs.transactions.first_tx_index,
                    &indexes.height.tx_index_count,
                    &fees.input_value,
                    |sats| !sats.is_max(),
                    exit,
                )?)
            })?;

        Ok(())
    }
}
