use std::time::{Duration, Instant};

use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, OutputType, Sats, StoredU64, TxInIndex, TxIndex, TxOutIndex, Version};
use tracing::info;
use vecdb::{AnyStoredVec, AnyVec, BinaryTransform, Exit, ReadableVec, VecIndex, WritableVec};

use super::{PegFlow, Vecs};
use crate::{
    inputs,
    internal::{SatsToCents, ValuePerBlock, ValuePerBlockCumulative},
    price,
};

const WRITE_INTERVAL: usize = 10_000;
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
        let outputs = &indexer.vecs().outputs;

        let dep_version = outputs.first_txout_index.version()
            + outputs.output_type.version()
            + outputs.value.version();

        validate_value_cum(&mut self.outputs_value, dep_version)?;
        validate_value_cum(&mut self.peg_pool.outputs_value, dep_version)?;
        validate_value_cum(&mut self.pegin.outputs_value, dep_version)?;
        self.pegin_count
            .validate_computed_version_or_reset(dep_version)?;

        let skip = value_cum_len(&self.outputs_value)
            .min(value_cum_len(&self.peg_pool.outputs_value))
            .min(value_cum_len(&self.pegin.outputs_value))
            .min(self.pegin_count.cumulative.height.len())
            .min(starting_lengths.height.to_usize());
        let end = outputs.first_txout_index.len();

        if skip < end {
            truncate_value_cum(&mut self.outputs_value, skip)?;
            truncate_value_cum(&mut self.peg_pool.outputs_value, skip)?;
            truncate_value_cum(&mut self.pegin.outputs_value, skip)?;
            self.pegin_count.truncate_if_needed_at(skip)?;

            let first_txout_indexes: Vec<TxOutIndex> = outputs.first_txout_index.collect_range_at(
                skip,
                (end + 1).min(outputs.first_txout_index.len()),
            );
            let total_outputs = outputs.value.len();
            let mut output_types_buf: Vec<OutputType> = Vec::new();
            let mut values_buf: Vec<Sats> = Vec::new();

            for local_idx in 0..(end - skip) {
                let first = first_txout_indexes[local_idx].to_usize();
                let next = first_txout_indexes
                    .get(local_idx + 1)
                    .copied()
                    .map(|i| i.to_usize())
                    .unwrap_or(total_outputs);

                outputs
                    .output_type
                    .collect_range_into_at(first, next, &mut output_types_buf);
                outputs
                    .value
                    .collect_range_into_at(first, next, &mut values_buf);

                let mut pool_value = Sats::ZERO;
                let mut pegin_value = Sats::ZERO;
                let mut pegin_count = 0u64;
                for (ot, val) in output_types_buf.iter().zip(values_buf.iter()) {
                    match ot {
                        OutputType::MWEBPegPool => pool_value += *val,
                        OutputType::MWEBPegIn => {
                            pegin_value += *val;
                            pegin_count += 1;
                        }
                        _ => {}
                    }
                }

                self.peg_pool.outputs_value.push_block_sats(pool_value);
                self.pegin.outputs_value.push_block_sats(pegin_value);
                self.outputs_value
                    .push_block_sats(pool_value + pegin_value);
                self.pegin_count.push_block(StoredU64::from(pegin_count));

                if (skip + local_idx + 1).is_multiple_of(WRITE_INTERVAL) {
                    let _lock = exit.lock();
                    write_output_flows(self)?;
                }
            }

            {
                let _lock = exit.lock();
                write_output_flows(self)?;
            }
        }

        self.peg_pool
            .outputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;
        self.pegin
            .outputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;
        self.outputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;

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
        let indexer_inputs = &indexer.vecs().inputs;

        let dep_version = indexer_inputs.first_txin_index.version()
            + indexer_inputs.output_type.version()
            + inputs.value.version();

        validate_value_cum(&mut self.inputs_value, dep_version)?;
        validate_value_cum(&mut self.peg_pool.inputs_value, dep_version)?;
        validate_value_cum(&mut self.pegin.inputs_value, dep_version)?;

        let skip = value_cum_len(&self.inputs_value)
            .min(value_cum_len(&self.peg_pool.inputs_value))
            .min(value_cum_len(&self.pegin.inputs_value))
            .min(starting_lengths.height.to_usize());
        let end = indexer_inputs.first_txin_index.len();

        if skip < end {
            truncate_value_cum(&mut self.inputs_value, skip)?;
            truncate_value_cum(&mut self.peg_pool.inputs_value, skip)?;
            truncate_value_cum(&mut self.pegin.inputs_value, skip)?;

            let first_txin_indexes: Vec<TxInIndex> = indexer_inputs
                .first_txin_index
                .collect_range_at(skip, (end + 1).min(indexer_inputs.first_txin_index.len()));
            let total_inputs = inputs.value.len();
            let mut input_types_buf: Vec<OutputType> = Vec::new();
            let mut values_buf: Vec<Sats> = Vec::new();

            for local_idx in 0..(end - skip) {
                let first = first_txin_indexes[local_idx].to_usize();
                let next = first_txin_indexes
                    .get(local_idx + 1)
                    .copied()
                    .map(|i| i.to_usize())
                    .unwrap_or(total_inputs);

                indexer_inputs.output_type.collect_range_into_at(
                    first,
                    next,
                    &mut input_types_buf,
                );
                inputs
                    .value
                    .collect_range_into_at(first, next, &mut values_buf);

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

                self.peg_pool.inputs_value.push_block_sats(pool_value);
                self.pegin.inputs_value.push_block_sats(pegin_value);
                self.inputs_value
                    .push_block_sats(pool_value + pegin_value);

                if (skip + local_idx + 1).is_multiple_of(WRITE_INTERVAL) {
                    let _lock = exit.lock();
                    write_input_flows(self)?;
                }
            }

            {
                let _lock = exit.lock();
                write_input_flows(self)?;
            }
        }

        self.peg_pool
            .inputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;
        self.pegin
            .inputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;
        self.inputs_value
            .compute_cents(starting_lengths.height, prices, exit)?;

        Ok(())
    }

    fn compute_pegout(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();
        let txs = &indexer.vecs().transactions;
        let outputs = &indexer.vecs().outputs;

        let dep_version = txs.first_tx_index.version()
            + txs.first_txout_index.version()
            + txs.is_hog_ex.version()
            + outputs.output_type.version()
            + outputs.value.version();

        validate_value_cum(&mut self.pegout_value, dep_version)?;
        self.pegout_count
            .validate_computed_version_or_reset(dep_version)?;

        let skip = value_cum_len(&self.pegout_value)
            .min(self.pegout_count.cumulative.height.len())
            .min(starting_lengths.height.to_usize());
        let end = txs.first_tx_index.len();
        if skip >= end {
            self.pegout_value
                .compute_rest(starting_lengths.height, prices, exit)?;
            return Ok(());
        }

        truncate_value_cum(&mut self.pegout_value, skip)?;
        self.pegout_count.truncate_if_needed_at(skip)?;

        let first_tx_indexes: Vec<TxIndex> = txs.first_tx_index.collect_range_at(skip, end);
        let txid_len = txs.txid.len();
        let total_outputs = outputs.value.len();

        info!(
            "Computing mweb peg-out for heights {skip}..={end} ({} blocks)...",
            end - skip
        );

        let mut output_types_buf: Vec<OutputType> = Vec::new();
        let mut values_buf: Vec<Sats> = Vec::new();
        let mut last_log = Instant::now();

        for (local_idx, first_tx) in first_tx_indexes.iter().enumerate() {
            let first = first_tx.to_usize();
            let next = first_tx_indexes
                .get(local_idx + 1)
                .copied()
                .unwrap_or_else(|| TxIndex::from(txid_len))
                .to_usize();

            let mut pegout_value = Sats::ZERO;
            let mut pegout_count = 0u64;
            let n_txs = next.saturating_sub(first);

            if n_txs > 0 {
                let hogex_flags = txs.is_hog_ex.collect_range_at(first, next);
                // +1 entry when the next block's first tx exists so the HogEx tx
                // (usually last in block) gets a correct local output bound.
                let fo_starts = if first + n_txs < txid_len {
                    txs.first_txout_index
                        .collect_range_at(first, next + 1)
                } else {
                    txs.first_txout_index.collect_range_at(first, next)
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

                    outputs.output_type.collect_range_into_at(
                        out_start,
                        out_end,
                        &mut output_types_buf,
                    );
                    outputs
                        .value
                        .collect_range_into_at(out_start, out_end, &mut values_buf);

                    for (ot, val) in output_types_buf.iter().zip(values_buf.iter()) {
                        if ot.is_mweb() {
                            continue;
                        }
                        pegout_value += *val;
                        pegout_count += 1;
                    }
                }
            }

            self.pegout_value.push_block_sats(pegout_value);
            self.pegout_count
                .push_block(StoredU64::from(pegout_count));

            let h = skip + local_idx;
            if h == skip
                || h + 1 == end
                || (h - skip) % PROGRESS_BLOCKS == 0
                || last_log.elapsed() >= PROGRESS_INTERVAL
            {
                let done = h - skip + 1;
                let total = end - skip;
                info!(
                    "mweb peg-out: height {h}/{} ({:.1}%)",
                    end - 1,
                    done as f64 / total as f64 * 100.0
                );
                last_log = Instant::now();
            }

            if (h + 1).is_multiple_of(WRITE_INTERVAL) {
                let _lock = exit.lock();
                write_value_cum(&mut self.pegout_value)?;
                self.pegout_count.write()?;
            }
        }

        {
            let _lock = exit.lock();
            write_value_cum(&mut self.pegout_value)?;
            self.pegout_count.write()?;
        }
        info!("mweb peg-out complete");

        self.pegout_value
            .compute_rest(starting_lengths.height, prices, exit)?;

        Ok(())
    }
}

fn compute_balance(
    flow: &mut PegFlow,
    max_height: Height,
    prices: &price::Vecs,
    exit: &Exit,
) -> Result<()> {
    compute_clamped_balance(
        &mut flow.balance,
        max_height,
        &flow.outputs_value.cumulative.sats.height,
        &flow.inputs_value.cumulative.sats.height,
        "MWEB peg flow balance: cumulative spent exceeded created at some heights; clamped to zero",
        prices,
        exit,
    )
}

fn compute_combined_balance(
    vecs: &mut Vecs,
    max_height: Height,
    prices: &price::Vecs,
    exit: &Exit,
) -> Result<()> {
    compute_clamped_balance(
        &mut vecs.balance,
        max_height,
        &vecs.outputs_value.cumulative.sats.height,
        &vecs.inputs_value.cumulative.sats.height,
        "MWEB balance: cumulative spent exceeded created at some heights; clamped to zero",
        prices,
        exit,
    )
}

fn compute_clamped_balance(
    balance: &mut ValuePerBlock,
    max_height: Height,
    created: &impl ReadableVec<Height, Sats>,
    spent: &impl ReadableVec<Height, Sats>,
    underflow_msg: &'static str,
    prices: &price::Vecs,
    exit: &Exit,
) -> Result<()> {
    let mut underflow_count: u64 = 0;
    let mut max_deficit = Sats::ZERO;
    balance.sats.height.compute_transform2(
        max_height,
        created,
        spent,
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
        tracing::warn!(underflow_count, ?max_deficit, "{underflow_msg}");
    }
    let ValuePerBlock { sats, cents, .. } = balance;
    cents.height.compute_transform2(
        max_height,
        &sats.height,
        &prices.spot.cents.height,
        |(h, sats, price, _)| (h, SatsToCents::apply(sats, price)),
        exit,
    )?;
    Ok(())
}

fn value_cum_len(vec: &ValuePerBlockCumulative) -> usize {
    vec.cumulative.sats.height.len()
}

fn validate_value_cum(vec: &mut ValuePerBlockCumulative, version: Version) -> Result<()> {
    vec.cumulative
        .sats
        .height
        .validate_computed_version_or_reset(version)?;
    Ok(())
}

fn truncate_value_cum(vec: &mut ValuePerBlockCumulative, skip: usize) -> Result<()> {
    vec.cumulative.sats.height.truncate_if_needed_at(skip)?;
    Ok(())
}

fn write_value_cum(vec: &mut ValuePerBlockCumulative) -> Result<()> {
    vec.cumulative.sats.height.write()?;
    Ok(())
}

fn write_output_flows(vecs: &mut Vecs) -> Result<()> {
    write_value_cum(&mut vecs.outputs_value)?;
    write_value_cum(&mut vecs.peg_pool.outputs_value)?;
    write_value_cum(&mut vecs.pegin.outputs_value)?;
    vecs.pegin_count.write()?;
    Ok(())
}

fn write_input_flows(vecs: &mut Vecs) -> Result<()> {
    write_value_cum(&mut vecs.inputs_value)?;
    write_value_cum(&mut vecs.peg_pool.inputs_value)?;
    write_value_cum(&mut vecs.pegin.inputs_value)?;
    Ok(())
}
