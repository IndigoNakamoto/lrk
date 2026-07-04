use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{FeeRate, OutPoint, OutputType, Sats, TxInIndex, TxIndex, VSize};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecIndex, WritableVec, unlikely};

use super::super::size;
use super::Vecs;
use crate::{indexes, inputs};

impl Vecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        spent: &inputs::SpentVecs,
        size_vecs: &size::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        self.input_value.compute_sum_from_indexes(
            starting_lengths.tx_index,
            &indexer.vecs.transactions.first_txin_index,
            &indexes.tx_index.input_count,
            &spent.value,
            exit,
        )?;
        self.output_value.compute_sum_from_indexes(
            starting_lengths.tx_index,
            &indexer.vecs.transactions.first_txout_index,
            &indexes.tx_index.output_count,
            &indexer.vecs.outputs.value,
            exit,
        )?;

        self.compute_transfer_input_value(indexer, spent, exit)?;

        self.compute_fees(indexer, indexes, size_vecs, exit)?;

        let vsize_source = &size_vecs.vsize.tx_index;
        let (r1, r2) = rayon::join(
            || {
                self.fee
                    .derive_from_with_skip(indexer, indexes, &starting_lengths, exit, 1)
            },
            || {
                self.effective_fee_rate.derive_from_with_skip_weighted(
                    indexer,
                    indexes,
                    &starting_lengths,
                    vsize_source,
                    exit,
                    1,
                )
            },
        );
        r1?;
        r2?;

        Ok(())
    }

    /// Per-tx sum of input values, excluding inputs whose prevout is a
    /// Litecoin MWEB output (peg-pool / peg-in). Coinbase txs are left as the
    /// `Sats::MAX` sentinel so downstream volume aggregation still skips them.
    fn compute_transfer_input_value(
        &mut self,
        indexer: &Indexer,
        spent: &inputs::SpentVecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        let dep_version = indexer.vecs.transactions.first_txin_index.version()
            + indexer.vecs.inputs.output_type.version()
            + spent.value.version();
        self.transfer_input_value
            .validate_computed_version_or_reset(dep_version)?;

        let target_tx = indexer.vecs.transactions.first_txin_index.len();
        let start = self
            .transfer_input_value
            .len()
            .min(starting_lengths.tx_index.to_usize());
        if start >= target_tx {
            return Ok(());
        }

        self.transfer_input_value
            .truncate_if_needed(TxIndex::from(start))?;

        let total_txin = spent.value.len();
        let output_type = &indexer.vecs.inputs.output_type;
        let first_txin_index = &indexer.vecs.transactions.first_txin_index;

        // Process many txs per batch so each input-vec chunk is read (and
        // pco-decompressed) exactly once. Reading one small per-tx range at a
        // time re-decompresses the chunk shared by every tx in it, turning this
        // into an O(txs × chunk_size) pass over ~400M txs (hours instead of
        // minutes). Batching reads each txin exactly once, matching the cost of
        // the streaming `input_value`/`output_value` sums above.
        const TX_BATCH: usize = 4_000_000;

        let mut type_buf: Vec<OutputType> = Vec::new();
        let mut value_buf: Vec<Sats> = Vec::new();

        let mut tx = start;
        while tx < target_tx {
            let tx_end = (tx + TX_BATCH).min(target_tx);

            // `firsts` spans [tx, tx_end]: the trailing entry (when present) is
            // the exclusive txin bound of the batch's last tx.
            let firsts: Vec<TxInIndex> =
                first_txin_index.collect_range_at(tx, (tx_end + 1).min(target_tx));
            let batch_txin_start = firsts[0].to_usize();
            let batch_txin_end = if tx_end < target_tx {
                firsts[tx_end - tx].to_usize()
            } else {
                total_txin
            };

            output_type.collect_range_into_at(batch_txin_start, batch_txin_end, &mut type_buf);
            spent
                .value
                .collect_range_into_at(batch_txin_start, batch_txin_end, &mut value_buf);

            for t in tx..tx_end {
                let fi = firsts[t - tx].to_usize();
                let next = if t + 1 < target_tx {
                    firsts[t + 1 - tx].to_usize()
                } else {
                    total_txin
                };
                let lo = fi - batch_txin_start;
                let hi = next - batch_txin_start;

                let mut sum = Sats::ZERO;
                for k in lo..hi {
                    let val = value_buf[k];
                    // Coinbase inputs carry `Sats::MAX`; keeping them makes the
                    // whole tx sum saturate to the sentinel (coinbase txs have a
                    // single input), which downstream volume filters skip.
                    if val.is_max() {
                        sum = Sats::MAX;
                        break;
                    }
                    if type_buf[k] != OutputType::MWEB {
                        sum += val;
                    }
                }

                self.transfer_input_value.push(sum);
            }

            let _lock = exit.lock();
            self.transfer_input_value.write()?;
            drop(_lock);

            tx = tx_end;
        }

        Ok(())
    }

    fn compute_fees(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        size_vecs: &size::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        let dep_version = self.input_value.version()
            + self.output_value.version()
            + size_vecs.vsize.tx_index.version();

        self.fee
            .tx_index
            .validate_computed_version_or_reset(dep_version)?;
        self.fee_rate
            .validate_computed_version_or_reset(dep_version)?;
        self.effective_fee_rate
            .tx_index
            .validate_computed_version_or_reset(dep_version)?;

        let target = self
            .input_value
            .len()
            .min(self.output_value.len())
            .min(size_vecs.vsize.tx_index.len());
        let min = self
            .fee
            .tx_index
            .len()
            .min(self.fee_rate.len())
            .min(self.effective_fee_rate.tx_index.len())
            .min(starting_lengths.tx_index.to_usize());

        if min >= target {
            return Ok(());
        }

        self.fee
            .tx_index
            .truncate_if_needed(starting_lengths.tx_index)?;
        self.fee_rate
            .truncate_if_needed(starting_lengths.tx_index)?;
        self.effective_fee_rate
            .tx_index
            .truncate_if_needed(starting_lengths.tx_index)?;

        let start_tx = self.fee.tx_index.len();
        let max_height = indexer.vecs.transactions.first_tx_index.len();

        let start_height = if start_tx == 0 {
            0
        } else {
            indexes
                .tx_heights
                .get_shared(TxIndex::from(start_tx))
                .unwrap()
                .to_usize()
        };

        for h in start_height..max_height {
            let first_tx: usize = indexer
                .vecs
                .transactions
                .first_tx_index
                .collect_one_at(h)
                .unwrap()
                .to_usize();
            let n = *indexes.height.tx_index_count.collect_one_at(h).unwrap() as usize;

            if first_tx + n > target {
                break;
            }

            // Batch read all per-tx data for this block
            let input_values = self.input_value.collect_range_at(first_tx, first_tx + n);
            let output_values = self.output_value.collect_range_at(first_tx, first_tx + n);
            let vsizes: Vec<VSize> = size_vecs
                .vsize
                .tx_index
                .collect_range_at(first_tx, first_tx + n);
            let txin_starts: Vec<TxInIndex> = indexer
                .vecs
                .transactions
                .first_txin_index
                .collect_range_at(first_tx, first_tx + n);
            let input_begin = txin_starts[0].to_usize();
            let input_end = if h + 1 < max_height {
                indexer
                    .vecs
                    .inputs
                    .first_txin_index
                    .collect_one_at(h + 1)
                    .unwrap()
                    .to_usize()
            } else {
                indexer.vecs.inputs.outpoint.len()
            };
            let outpoints: Vec<OutPoint> = indexer
                .vecs
                .inputs
                .outpoint
                .collect_range_at(input_begin, input_end);

            // Compute fee + fee_rate per tx
            let mut fees = Vec::with_capacity(n);
            for j in 0..n {
                let fee = if unlikely(input_values[j].is_max()) {
                    Sats::ZERO
                } else {
                    input_values[j] - output_values[j]
                };
                self.fee.tx_index.push(fee);
                self.fee_rate.push(FeeRate::from((fee, vsizes[j])));
                fees.push(fee);
            }

            // Effective fee rate via same-block CPFP clustering
            let effective = cluster_fee_rates(
                &txin_starts,
                &outpoints,
                input_begin,
                first_tx,
                &fees,
                &vsizes,
            );
            for rate in effective {
                self.effective_fee_rate.tx_index.push(rate);
            }

            if h % 1_000 == 0 {
                let _lock = exit.lock();
                self.fee.tx_index.write()?;
                self.fee_rate.write()?;
                self.effective_fee_rate.tx_index.write()?;
            }
        }

        let _lock = exit.lock();
        self.fee.tx_index.write()?;
        self.fee_rate.write()?;
        self.effective_fee_rate.tx_index.write()?;

        Ok(())
    }
}

/// Clusters same-block parent-child txs and computes effective fee rate per cluster.
fn cluster_fee_rates(
    txin_starts: &[TxInIndex],
    outpoints: &[OutPoint],
    outpoint_base: usize,
    first_tx: usize,
    fees: &[Sats],
    vsizes: &[VSize],
) -> Vec<FeeRate> {
    let n = fees.len();
    let mut parent: Vec<usize> = (0..n).collect();

    for j in 1..n {
        let start = txin_starts[j].to_usize() - outpoint_base;
        let end = if j + 1 < txin_starts.len() {
            txin_starts[j + 1].to_usize() - outpoint_base
        } else {
            outpoints.len()
        };

        for op in &outpoints[start..end] {
            if op.is_coinbase() {
                continue;
            }
            let parent_tx = op.tx_index().to_usize();
            if parent_tx >= first_tx && parent_tx < first_tx + n {
                union(&mut parent, j, parent_tx - first_tx);
            }
        }
    }

    let mut cluster_fee = vec![Sats::ZERO; n];
    let mut cluster_vsize = vec![VSize::from(0u64); n];
    for j in 0..n {
        let root = find(&mut parent, j);
        cluster_fee[root] += fees[j];
        cluster_vsize[root] += vsizes[j];
    }

    (0..n)
        .map(|j| {
            let root = find(&mut parent, j);
            FeeRate::from((cluster_fee[root], cluster_vsize[root]))
        })
        .collect()
}

fn find(parent: &mut [usize], mut i: usize) -> usize {
    while parent[i] != i {
        parent[i] = parent[parent[i]];
        i = parent[i];
    }
    i
}

fn union(parent: &mut [usize], a: usize, b: usize) {
    let ra = find(parent, a);
    let rb = find(parent, b);
    if ra != rb {
        parent[ra] = rb;
    }
}
