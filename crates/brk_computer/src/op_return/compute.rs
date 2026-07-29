use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, OpReturnKind, PartsPerMillion32, Sats, StoredU64, VSize};
use vecdb::{AnyVec, Exit, ReadableVec, VecIndex};

use super::{Breakdown, Vecs, vecs::Totals};
use crate::{
    blocks,
    internal::{
        PerBlockCumulativeRolling, PercentCumulativeRolling, PercentPerBlock, RatioSats, RatioU64,
        ValuePerBlockCumulativeRolling,
    },
    transactions,
};

const KIND_COUNT: usize = OpReturnKind::Unknown as usize + 1;
const OLD_STANDARD_MAX_POST_OP_RETURN_BYTES: u64 = 82;
const WRITE_INTERVAL: usize = 10_000;

#[derive(Clone, Copy, Default)]
struct PolicyTotals {
    pre_v30_standard: Totals,
    pre_v30_nonstandard: Totals,
    oversized: Totals,
    multiple: Totals,
}

#[derive(Clone, Copy, Default)]
struct Carrier {
    kinds: u32,
    output_count: u64,
    data_bytes: u64,
    oversized_output_count: u64,
    oversized_data_bytes: u64,
    vsize: VSize,
    fees: Sats,
}

impl Carrier {
    fn add_output(&mut self, kind: OpReturnKind, data_bytes: u64) {
        self.kinds |= kind_bit(kind);
        self.output_count += 1;
        self.data_bytes += data_bytes;
        if data_bytes > OLD_STANDARD_MAX_POST_OP_RETURN_BYTES {
            self.oversized_output_count += 1;
            self.oversized_data_bytes += data_bytes;
        }
    }
}

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        fees: &transactions::FeesVecs,
        blocks: &blocks::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_lengths = indexer.safe_lengths();
        let raw = &indexer.vecs.op_return;
        let txs = &indexer.vecs.transactions;
        let version = raw.first_index.version()
            + raw.to_tx_index.version()
            + raw.kind.version()
            + raw.post_op_return_bytes.version()
            + txs.weight.version()
            + fees.fee.tx_index.version();

        self.validate_and_truncate(version, starting_lengths.height)?;

        let skip = self.min_len();
        let end = raw.first_index.len();
        if skip < end {
            self.truncate_if_needed_at(skip)?;

            let op_return_len = raw.to_tx_index.len();
            let mut tx_cursor = raw.to_tx_index.cursor();
            let mut kind_cursor = raw.kind.cursor();
            let mut post_op_return_bytes = raw.post_op_return_bytes.cursor();
            let mut first_index_cursor = raw.first_index.cursor();
            let mut weight_cursor = txs.weight.cursor();
            let mut fee_cursor = fees.fee.tx_index.cursor();
            first_index_cursor.advance(skip);
            let mut start = first_index_cursor.next().unwrap().to_usize();

            for height in skip..end {
                let block_end = if height + 1 < end {
                    first_index_cursor.next().unwrap().to_usize()
                } else {
                    op_return_len
                };

                tx_cursor.advance(start - tx_cursor.position());
                kind_cursor.advance(start - kind_cursor.position());
                post_op_return_bytes.advance(start - post_op_return_bytes.position());

                let mut total = Totals::default();
                let mut by_kind = [Totals::default(); KIND_COUNT];
                let mut policy = PolicyTotals::default();
                let mut current_tx = None;
                let mut carrier = Carrier::default();

                for _ in start..block_end {
                    let tx_index = tx_cursor.next().unwrap();
                    let kind = kind_cursor.next().unwrap();
                    let bytes = u32::from(post_op_return_bytes.next().unwrap()) as u64;
                    let kind_index = kind as usize;

                    if current_tx != Some(tx_index) {
                        finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);
                        current_tx = Some(tx_index);
                        carrier = Carrier::default();

                        let tx_position = tx_index.to_usize();
                        weight_cursor.advance(tx_position - weight_cursor.position());
                        carrier.vsize = VSize::from(weight_cursor.next().unwrap());
                        fee_cursor.advance(tx_position - fee_cursor.position());
                        carrier.fees = fee_cursor.next().unwrap();
                    }

                    total.data_bytes += bytes;
                    by_kind[kind_index].output_count += 1;
                    by_kind[kind_index].data_bytes += bytes;
                    carrier.add_output(kind, bytes);
                }

                finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

                self.total.push(total);
                for (kind, metrics) in self.by_kind.iter_typed_mut() {
                    metrics.push(by_kind[kind as usize]);
                }
                self.policy.pre_v30_standard.push(policy.pre_v30_standard);
                self.policy
                    .pre_v30_nonstandard
                    .push(policy.pre_v30_nonstandard);
                self.policy.oversized.push(policy.oversized);
                self.policy.multiple.push(policy.multiple);

                if (height + 1).is_multiple_of(WRITE_INTERVAL) {
                    let _lock = exit.lock();
                    self.write()?;
                }
                start = block_end;
            }

            let _lock = exit.lock();
            self.write()?;
        }

        let block_size = &blocks.size.size.cumulative.height;
        compute_data_share(
            starting_lengths.height,
            &mut self.total.chain_share,
            &self.total.metrics.data_bytes.cumulative.height,
            block_size,
            exit,
        )?;
        let total_data = &self.total.metrics.data_bytes.cumulative.height;
        for breakdown in self.by_kind.iter_mut() {
            compute_breakdown_data_shares(
                starting_lengths.height,
                breakdown,
                total_data,
                block_size,
                exit,
            )?;
        }
        for policy in self.policy.iter_mut() {
            compute_breakdown_data_shares(
                starting_lengths.height,
                policy,
                total_data,
                block_size,
                exit,
            )?;
        }

        Ok(())
    }

    pub(crate) fn compute_fee_shares(
        &mut self,
        chain_fees: &ValuePerBlockCumulativeRolling,
        max_from: Height,
        exit: &Exit,
    ) -> Result<()> {
        compute_fee_share(
            max_from,
            &mut self.total.fee_share,
            &self.total.metrics.fees,
            chain_fees,
            exit,
        )?;
        for breakdown in self.by_kind.iter_mut() {
            compute_fee_share(
                max_from,
                &mut breakdown.fee_share,
                &breakdown.metrics.total.fees,
                chain_fees,
                exit,
            )?;
        }
        for policy in self.policy.iter_mut() {
            compute_fee_share(
                max_from,
                &mut policy.fee_share,
                &policy.metrics.total.fees,
                chain_fees,
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

fn compute_fee_share(
    max_from: Height,
    target: &mut PercentCumulativeRolling<PartsPerMillion32>,
    numerator: &PerBlockCumulativeRolling<Sats>,
    denominator: &ValuePerBlockCumulativeRolling,
    exit: &Exit,
) -> Result<()> {
    target.compute_binary::<Sats, Sats, RatioSats<PartsPerMillion32>, _, _, _, _>(
        max_from,
        &numerator.cumulative.height,
        &denominator.cumulative.sats.height,
        numerator.sum.as_array().map(|w| &w.height),
        denominator.sum.as_array().map(|w| &w.sats.height),
        exit,
    )
}

fn compute_breakdown_data_shares(
    max_from: Height,
    breakdown: &mut Breakdown,
    total_data: &impl ReadableVec<Height, StoredU64>,
    block_size: &impl ReadableVec<Height, StoredU64>,
    exit: &Exit,
) -> Result<()> {
    let data = &breakdown.metrics.total.data_bytes.cumulative.height;
    compute_data_share(max_from, &mut breakdown.chain_share, data, block_size, exit)?;
    compute_data_share(max_from, &mut breakdown.data_share, data, total_data, exit)
}

fn compute_data_share(
    max_from: Height,
    target: &mut PercentPerBlock<PartsPerMillion32>,
    data: &impl ReadableVec<Height, StoredU64>,
    block_size: &impl ReadableVec<Height, StoredU64>,
    exit: &Exit,
) -> Result<()> {
    target.compute_binary::<StoredU64, StoredU64, RatioU64<PartsPerMillion32>>(
        max_from, data, block_size, exit,
    )
}

fn finalize_transaction(
    total: &mut Totals,
    by_kind: &mut [Totals; KIND_COUNT],
    policy: &mut PolicyTotals,
    carrier: Carrier,
) {
    if carrier.output_count == 0 {
        return;
    }

    add_carrier(total, carrier);
    let mut kinds = carrier.kinds;
    while kinds != 0 {
        let kind_index = kinds.trailing_zeros() as usize;
        add_carrier(&mut by_kind[kind_index], carrier);
        kinds &= kinds - 1;
    }

    if carrier.oversized_output_count > 0 {
        policy.oversized.output_count += carrier.oversized_output_count;
        policy.oversized.data_bytes += carrier.oversized_data_bytes;
        add_carrier(&mut policy.oversized, carrier);
    }

    if carrier.output_count > 1 {
        policy.multiple.output_count += carrier.output_count;
        policy.multiple.data_bytes += carrier.data_bytes;
        add_carrier(&mut policy.multiple, carrier);
    }

    if carrier.oversized_output_count > 0 || carrier.output_count > 1 {
        policy.pre_v30_nonstandard.output_count += carrier.output_count;
        policy.pre_v30_nonstandard.data_bytes += carrier.data_bytes;
        add_carrier(&mut policy.pre_v30_nonstandard, carrier);
    } else {
        policy.pre_v30_standard.output_count += carrier.output_count;
        policy.pre_v30_standard.data_bytes += carrier.data_bytes;
        add_carrier(&mut policy.pre_v30_standard, carrier);
    }
}

fn add_carrier(metrics: &mut Totals, carrier: Carrier) {
    metrics.tx_count += 1;
    metrics.tx_vsize += carrier.vsize;
    metrics.fees += carrier.fees;
}

const fn kind_bit(kind: OpReturnKind) -> u32 {
    1_u32 << kind as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn multiple_kinds_count_one_total_carrier() {
        let mut total = Totals::default();
        let mut by_kind = [Totals::default(); KIND_COUNT];
        let mut policy = PolicyTotals::default();
        let mut carrier = Carrier {
            vsize: VSize::new(100),
            fees: Sats::new(500),
            ..Carrier::default()
        };
        carrier.add_output(OpReturnKind::Runes, 15);
        carrier.add_output(OpReturnKind::Omni, 15);

        finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

        assert_eq!(total.tx_count, 1);
        assert_eq!(by_kind[OpReturnKind::Runes as usize].tx_count, 1);
        assert_eq!(by_kind[OpReturnKind::Omni as usize].tx_count, 1);
        assert_eq!(total.fees, Sats::new(500));
        assert_eq!(by_kind[OpReturnKind::Runes as usize].fees, Sats::new(500));
        assert_eq!(by_kind[OpReturnKind::Omni as usize].fees, Sats::new(500));
        assert_eq!(policy.multiple.fees, Sats::new(500));
        assert_eq!(policy.pre_v30_nonstandard.fees, Sats::new(500));
        assert_eq!(policy.multiple.output_count, 2);
        assert_eq!(policy.pre_v30_nonstandard.tx_count, 1);
        assert_eq!(policy.oversized.tx_count, 0);
        assert_eq!(policy.pre_v30_standard.tx_count, 0);
    }

    #[test]
    fn oversized_output_marks_pre_v30_nonstandard_once() {
        let mut total = Totals::default();
        let mut by_kind = [Totals::default(); KIND_COUNT];
        let mut policy = PolicyTotals::default();
        let mut carrier = Carrier {
            vsize: VSize::new(120),
            ..Carrier::default()
        };
        carrier.add_output(OpReturnKind::Unknown, 83);

        finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

        assert_eq!(policy.oversized.output_count, 1);
        assert_eq!(policy.oversized.tx_vsize, VSize::new(120));
        assert_eq!(policy.pre_v30_nonstandard.tx_count, 1);
        assert_eq!(policy.multiple.tx_count, 0);
        assert_eq!(policy.pre_v30_standard.tx_count, 0);
    }

    #[test]
    fn standard_output_is_recorded_directly() {
        let mut total = Totals::default();
        let mut by_kind = [Totals::default(); KIND_COUNT];
        let mut policy = PolicyTotals::default();
        let mut carrier = Carrier {
            vsize: VSize::new(100),
            ..Carrier::default()
        };
        carrier.add_output(OpReturnKind::Runes, 15);

        finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

        assert_eq!(policy.pre_v30_standard.output_count, 1);
        assert_eq!(policy.pre_v30_standard.data_bytes, 15);
        assert_eq!(policy.pre_v30_standard.tx_count, 1);
        assert_eq!(policy.pre_v30_standard.tx_vsize, VSize::new(100));
        assert_eq!(policy.pre_v30_nonstandard.tx_count, 0);
    }
}
