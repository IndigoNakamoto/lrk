use std::time::{Duration, Instant};

use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, OutputType, Sats, StoredU64, TxInIndex, TxIndex, TxOutIndex};
use tracing::info;
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecIndex, WritableVec};

use super::{PegFlow, Vecs};
use crate::{inputs, price};

const PROGRESS_BLOCKS: usize = 50_000;
const PROGRESS_INTERVAL: Duration = Duration::from_secs(30);

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        inputs: &inputs::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        self.compute_output_flows(indexer, prices, exit)?;
        self.compute_input_flows(indexer, inputs, prices, exit)?;
        self.compute_pegin_count(indexer, exit)?;
        self.compute_pegout(indexer, prices, exit)?;

        compute_balance(
            &mut self.peg_pool,
            starting_lengths.height,
            prices,
            exit,
        )?;
        compute_balance(&mut self.pegin, starting_lengths.height, prices, exit)?;
        compute_combined_balance(self, starting_lengths.height, prices, exit)?;

        Ok(())
    }

    fn compute_output_flows(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        self.outputs_value
            .compute_with(starting_lengths.height, prices, exit, |height_vec| {
                self.peg_pool.outputs_value.block.sats.truncate_if_needed(starting_lengths.height)?;
                self.pegin.outputs_value.block.sats.truncate_if_needed(starting_lengths.height)?;

                let dep_version = indexer.vecs.outputs.first_txout_index.version()
                    + indexer.vecs.outputs.output_type.version()
                    + indexer.vecs.outputs.value.version();
                height_vec.validate_computed_version_or_reset(dep_version)?;

                let target_len = indexer.vecs.outputs.first_txout_index.len();
                if target_len == 0 {
                    return Ok(());
                }
                let target_height = Height::from(target_len - 1);

                let current_len = height_vec.len();
                let starting_height =
                    Height::from(current_len.min(starting_lengths.height.to_usize()));
                if starting_height > target_height {
                    return Ok(());
                }

                let first_txout_indexes: Vec<TxOutIndex> =
                    indexer.vecs.outputs.first_txout_index.collect_range_at(
                        starting_height.to_usize(),
                        target_height.to_usize()
                            + 2.min(indexer.vecs.outputs.first_txout_index.len()),
                    );

                let mut output_types_buf: Vec<OutputType> = Vec::new();
                let mut values_buf: Vec<Sats> = Vec::new();

                height_vec.truncate_if_needed(starting_height)?;

                for h in starting_height.to_usize()..=target_height.to_usize() {
                    let local_idx = h - starting_height.to_usize();

                    let first_txout_index = first_txout_indexes[local_idx];
                    let next_first_txout_index = first_txout_indexes
                        .get(local_idx + 1)
                        .copied()
                        .unwrap_or_else(|| TxOutIndex::from(indexer.vecs.outputs.value.len()));

                    let out_start = first_txout_index.to_usize();
                    let out_end = next_first_txout_index.to_usize();

                    indexer.vecs.outputs.output_type.collect_range_into_at(
                        out_start,
                        out_end,
                        &mut output_types_buf,
                    );
                    indexer.vecs.outputs.value.collect_range_into_at(
                        out_start,
                        out_end,
                        &mut values_buf,
                    );

                    let mut pool_value = Sats::ZERO;
                    let mut pegin_value = Sats::ZERO;
                    for (ot, val) in output_types_buf.iter().zip(values_buf.iter()) {
                        match ot {
                            OutputType::MWEBPegPool => pool_value += *val,
                            OutputType::MWEBPegIn => pegin_value += *val,
                            _ => {}
                        }
                    }

                    self.peg_pool.outputs_value.block.sats.push(pool_value);
                    self.pegin.outputs_value.block.sats.push(pegin_value);
                    height_vec.push(pool_value + pegin_value);
                }

                self.peg_pool.outputs_value.block.sats.write()?;
                self.pegin.outputs_value.block.sats.write()?;
                height_vec.write()?;
                Ok(())
            })?;

        self.peg_pool
            .outputs_value
            .compute(prices, starting_lengths.height, exit)?;
        self.pegin
            .outputs_value
            .compute(prices, starting_lengths.height, exit)?;
        self.outputs_value
            .compute(prices, starting_lengths.height, exit)?;

        Ok(())
    }

    fn compute_input_flows(
        &mut self,
        indexer: &Indexer,
        inputs: &inputs::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        self.inputs_value
            .compute_with(starting_lengths.height, prices, exit, |height_vec| {
                self.peg_pool.inputs_value.block.sats.truncate_if_needed(starting_lengths.height)?;
                self.pegin.inputs_value.block.sats.truncate_if_needed(starting_lengths.height)?;

                let dep_version = indexer.vecs.inputs.first_txin_index.version()
                    + indexer.vecs.inputs.output_type.version()
                    + inputs.spent.value.version();
                height_vec.validate_computed_version_or_reset(dep_version)?;

                let target_len = indexer.vecs.inputs.first_txin_index.len();
                if target_len == 0 {
                    return Ok(());
                }
                let target_height = Height::from(target_len - 1);

                let current_len = height_vec.len();
                let starting_height =
                    Height::from(current_len.min(starting_lengths.height.to_usize()));
                if starting_height > target_height {
                    return Ok(());
                }

                let first_txin_indexes: Vec<TxInIndex> =
                    indexer.vecs.inputs.first_txin_index.collect_range_at(
                        starting_height.to_usize(),
                        target_height.to_usize()
                            + 2.min(indexer.vecs.inputs.first_txin_index.len()),
                    );

                let mut input_types_buf: Vec<OutputType> = Vec::new();
                let mut values_buf: Vec<Sats> = Vec::new();

                height_vec.truncate_if_needed(starting_height)?;

                for h in starting_height.to_usize()..=target_height.to_usize() {
                    let local_idx = h - starting_height.to_usize();

                    let first_txin_index = first_txin_indexes[local_idx];
                    let next_first_txin_index = first_txin_indexes
                        .get(local_idx + 1)
                        .copied()
                        .unwrap_or_else(|| TxInIndex::from(inputs.spent.value.len()));

                    let in_start = first_txin_index.to_usize();
                    let in_end = next_first_txin_index.to_usize();

                    indexer.vecs.inputs.output_type.collect_range_into_at(
                        in_start,
                        in_end,
                        &mut input_types_buf,
                    );
                    inputs
                        .spent
                        .value
                        .collect_range_into_at(in_start, in_end, &mut values_buf);

                    let mut pool_value = Sats::ZERO;
                    let mut pegin_value = Sats::ZERO;
                    for (it, val) in input_types_buf.iter().zip(values_buf.iter()) {
                        if val.is_max() {
                            continue;
                        }
                        match it {
                            OutputType::MWEBPegPool => pool_value += *val,
                            OutputType::MWEBPegIn => pegin_value += *val,
                            _ => {}
                        }
                    }

                    self.peg_pool.inputs_value.block.sats.push(pool_value);
                    self.pegin.inputs_value.block.sats.push(pegin_value);
                    height_vec.push(pool_value + pegin_value);
                }

                self.peg_pool.inputs_value.block.sats.write()?;
                self.pegin.inputs_value.block.sats.write()?;
                height_vec.write()?;
                Ok(())
            })?;

        self.peg_pool
            .inputs_value
            .compute(prices, starting_lengths.height, exit)?;
        self.pegin
            .inputs_value
            .compute(prices, starting_lengths.height, exit)?;
        self.inputs_value
            .compute(prices, starting_lengths.height, exit)?;

        Ok(())
    }

    fn compute_pegin_count(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        let dep_version = indexer.vecs.outputs.first_txout_index.version()
            + indexer.vecs.outputs.output_type.version();
        self.pegin_count
            .block
            .validate_computed_version_or_reset(dep_version)?;

        self.pegin_count.compute(starting_lengths.height, exit, |height_vec| {
            let target_len = indexer.vecs.outputs.first_txout_index.len();
            if target_len == 0 {
                return Ok(());
            }
            let target_height = Height::from(target_len - 1);

            let current_len = height_vec.len();
            let starting_height =
                Height::from(current_len.min(starting_lengths.height.to_usize()));
            if starting_height > target_height {
                return Ok(());
            }

            let first_txout_indexes: Vec<TxOutIndex> =
                indexer.vecs.outputs.first_txout_index.collect_range_at(
                    starting_height.to_usize(),
                    target_height.to_usize() + 2.min(indexer.vecs.outputs.first_txout_index.len()),
                );

            let mut output_types_buf: Vec<OutputType> = Vec::new();

            height_vec.truncate_if_needed(starting_height)?;

            for h in starting_height.to_usize()..=target_height.to_usize() {
                let local_idx = h - starting_height.to_usize();

                let first_txout_index = first_txout_indexes[local_idx];
                let next_first_txout_index = first_txout_indexes
                    .get(local_idx + 1)
                    .copied()
                    .unwrap_or_else(|| TxOutIndex::from(indexer.vecs.outputs.output_type.len()));

                indexer.vecs.outputs.output_type.collect_range_into_at(
                    first_txout_index.to_usize(),
                    next_first_txout_index.to_usize(),
                    &mut output_types_buf,
                );

                let count = output_types_buf
                    .iter()
                    .filter(|ot| **ot == OutputType::MWEBPegIn)
                    .count();
                height_vec.push(StoredU64::from(count as u64));
            }

            height_vec.write()?;
            Ok(())
        })
    }

    fn compute_pegout(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        let dep_version = indexer.vecs.transactions.first_tx_index.version()
            + indexer.vecs.transactions.first_txout_index.version()
            + indexer.vecs.transactions.is_hog_ex.version()
            + indexer.vecs.outputs.output_type.version()
            + indexer.vecs.outputs.value.version();

        self.pegout_value
            .inner
            .block
            .sats
            .validate_computed_version_or_reset(dep_version)?;
        self.pegout_count
            .block
            .validate_computed_version_or_reset(dep_version)?;

        let sats_vec = &mut self.pegout_value.block.sats;
        let count_vec = &mut self.pegout_count.block;

        let target_len = indexer.vecs.transactions.first_tx_index.len();
        if target_len == 0 {
            return Ok(());
        }
        let target_height = Height::from(target_len - 1);

        let current_len = sats_vec.len();
        let starting_height =
            Height::from(current_len.min(starting_lengths.height.to_usize()));
        if starting_height > target_height {
            return Ok(());
        }

        let first_tx_indexes: Vec<TxIndex> = indexer.vecs.transactions.first_tx_index.collect_range_at(
            starting_height.to_usize(),
            target_height.to_usize() + 2.min(indexer.vecs.transactions.first_tx_index.len()),
        );

        sats_vec.truncate_if_needed(starting_height)?;
        count_vec.truncate_if_needed(starting_height)?;

        let start = starting_height.to_usize();
        let end = target_height.to_usize();
        info!(
            "Computing mweb peg-out for heights {start}..={end} ({} blocks)...",
            end - start + 1
        );

        let mut output_types_buf: Vec<OutputType> = Vec::new();
        let mut values_buf: Vec<Sats> = Vec::new();
        let mut last_log = Instant::now();
        let total_outputs = indexer.vecs.outputs.value.len();

        for h in start..=end {
            let local_idx = h - start;
            let first_tx = first_tx_indexes[local_idx].to_usize();
            let next_tx = first_tx_indexes
                .get(local_idx + 1)
                .copied()
                .unwrap_or_else(|| TxIndex::from(indexer.vecs.transactions.txid.len()))
                .to_usize();

            let mut pegout_value = Sats::ZERO;
            let mut pegout_count = 0u64;
            let n_txs = next_tx.saturating_sub(first_tx);

            if n_txs > 0 {
                let hogex_flags =
                    indexer.vecs.transactions.is_hog_ex.collect_range_at(first_tx, next_tx);
                // +1 entry when the next block's first tx exists so the HogEx tx
                // (usually last in block) gets a correct local output bound.
                let fo_starts = if local_idx + 1 < first_tx_indexes.len() {
                    indexer
                        .vecs
                        .transactions
                        .first_txout_index
                        .collect_range_at(first_tx, next_tx + 1)
                } else {
                    indexer
                        .vecs
                        .transactions
                        .first_txout_index
                        .collect_range_at(first_tx, next_tx)
                };

                for (i, flag) in hogex_flags.iter().enumerate() {
                    if !flag.is_true() {
                        continue;
                    }
                    let out_start = fo_starts[i].to_usize();
                    let out_end = if i + 1 < fo_starts.len() {
                        fo_starts[i + 1].to_usize()
                    } else {
                        total_outputs
                    };

                    indexer.vecs.outputs.output_type.collect_range_into_at(
                        out_start,
                        out_end,
                        &mut output_types_buf,
                    );
                    indexer.vecs.outputs.value.collect_range_into_at(
                        out_start,
                        out_end,
                        &mut values_buf,
                    );

                    for (ot, val) in output_types_buf.iter().zip(values_buf.iter()) {
                        if ot.is_mweb() {
                            continue;
                        }
                        pegout_value += *val;
                        pegout_count += 1;
                    }
                }
            }

            sats_vec.push(pegout_value);
            count_vec.push(StoredU64::from(pegout_count));

            if h == start
                || h == end
                || (h - start) % PROGRESS_BLOCKS == 0
                || last_log.elapsed() >= PROGRESS_INTERVAL
            {
                let done = h - start + 1;
                let total = end - start + 1;
                info!(
                    "mweb peg-out: height {h}/{end} ({:.1}%)",
                    done as f64 / total as f64 * 100.0
                );
                last_log = Instant::now();
            }
        }

        sats_vec.write()?;
        count_vec.write()?;
        info!("mweb peg-out complete");

        self.pegout_value
            .compute_rest(starting_lengths.height, prices, exit)?;
        self.pegout_count
            .compute_rest(starting_lengths.height, exit)?;

        Ok(())
    }
}

fn compute_balance(
    flow: &mut PegFlow,
    max_height: Height,
    prices: &price::Vecs,
    exit: &Exit,
) -> Result<()> {
    let mut underflow_count: u64 = 0;
    let mut max_deficit = Sats::ZERO;
    flow.balance.sats.height.compute_transform2(
        max_height,
        &flow.outputs_value.cumulative.sats.height,
        &flow.inputs_value.cumulative.sats.height,
        |(h, created, spent, _)| {
            let balance = if created >= spent {
                created - spent
            } else {
                underflow_count += 1;
                let deficit = spent - created;
                if deficit > max_deficit {
                    max_deficit = deficit;
                }
                Sats::ZERO
            };
            (h, balance)
        },
        exit,
    )?;
    if underflow_count > 0 {
        tracing::warn!(
            underflow_count,
            ?max_deficit,
            "MWEB peg flow balance: cumulative spent exceeded created at some heights; clamped to zero"
        );
    }
    flow.balance.compute(prices, max_height, exit)
}

fn compute_combined_balance(
    vecs: &mut Vecs,
    max_height: Height,
    prices: &price::Vecs,
    exit: &Exit,
) -> Result<()> {
    let mut underflow_count: u64 = 0;
    let mut max_deficit = Sats::ZERO;
    vecs.balance.sats.height.compute_transform2(
        max_height,
        &vecs.outputs_value.cumulative.sats.height,
        &vecs.inputs_value.cumulative.sats.height,
        |(h, created, spent, _)| {
            let balance = if created >= spent {
                created - spent
            } else {
                underflow_count += 1;
                let deficit = spent - created;
                if deficit > max_deficit {
                    max_deficit = deficit;
                }
                Sats::ZERO
            };
            (h, balance)
        },
        exit,
    )?;
    if underflow_count > 0 {
        tracing::warn!(
            underflow_count,
            ?max_deficit,
            "MWEB balance: cumulative spent exceeded created at some heights; clamped to zero"
        );
    }
    vecs.balance.compute(prices, max_height, exit)
}
