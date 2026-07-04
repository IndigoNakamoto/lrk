use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, OutputType, Sats, TxInIndex, TxOutIndex};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecIndex, WritableVec};

use super::Vecs;
use crate::{inputs, price};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        inputs: &inputs::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();

        // MWEB outputs created per block (peg-ins + freshly-created peg-pool).
        self.outputs_value
            .compute_with(starting_lengths.height, prices, exit, |height_vec| {
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

                    let mut mweb_value = Sats::ZERO;
                    for (ot, val) in output_types_buf.iter().zip(values_buf.iter()) {
                        if *ot == OutputType::MWEB {
                            mweb_value += *val;
                        }
                    }

                    height_vec.push(mweb_value);
                }

                height_vec.write()?;
                Ok(())
            })?;

        // MWEB outputs spent per block (consumed peg-ins + prior peg-pool
        // re-spent by the HogEx integration tx).
        self.inputs_value
            .compute_with(starting_lengths.height, prices, exit, |height_vec| {
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

                    let mut mweb_value = Sats::ZERO;
                    for (it, val) in input_types_buf.iter().zip(values_buf.iter()) {
                        // Coinbase inputs carry a `Sats::MAX` sentinel but are
                        // typed `Unknown`, so they never match here. The extra
                        // `is_max` guard defensively drops any stray sentinel so
                        // it can't inflate the spent total past created.
                        if *it == OutputType::MWEB && !val.is_max() {
                            mweb_value += *val;
                        }
                    }

                    height_vec.push(mweb_value);
                }

                height_vec.write()?;
                Ok(())
            })?;

        // Pegged balance = cumulative MWEB outputs created − cumulative spent.
        // The pool balance is a non-negative unsigned quantity, so we clamp at
        // zero rather than using `compute_subtract` (which aborts on any
        // per-height underflow). Created ≥ spent should hold at every height,
        // but a saturating floor keeps a stray accounting artifact from taking
        // down the whole compute run.
        let mut underflow_count: u64 = 0;
        let mut max_deficit = Sats::ZERO;
        self.balance.sats.height.compute_transform2(
            starting_lengths.height,
            &self.outputs_value.cumulative.sats.height,
            &self.inputs_value.cumulative.sats.height,
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
        self.balance.compute(prices, starting_lengths.height, exit)?;

        Ok(())
    }
}
