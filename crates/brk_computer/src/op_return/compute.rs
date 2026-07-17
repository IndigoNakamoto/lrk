use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{OpReturnKind, VSize};
use vecdb::{AnyVec, Exit, ReadableVec, VecIndex};

use super::{Vecs, vecs::Totals};

const KIND_COUNT: usize = OpReturnKind::Unknown as usize + 1;
const OLD_STANDARD_MAX_POST_OP_RETURN_BYTES: u64 = 82;
const WRITE_INTERVAL: usize = 10_000;

#[derive(Clone, Copy, Default)]
struct PolicyTotals {
    standard: Totals,
    oversized: Totals,
    multiple: Totals,
    pre_v30_nonstandard: Totals,
}

#[derive(Clone, Copy, Default)]
struct Carrier {
    kinds: u32,
    output_count: u64,
    post_op_return_bytes: u64,
    oversized_output_count: u64,
    oversized_post_op_return_bytes: u64,
    vsize: VSize,
}

impl Carrier {
    fn add_output(&mut self, kind: OpReturnKind, post_op_return_bytes: u64) {
        self.kinds |= kind_bit(kind);
        self.output_count += 1;
        self.post_op_return_bytes += post_op_return_bytes;
        if post_op_return_bytes > OLD_STANDARD_MAX_POST_OP_RETURN_BYTES {
            self.oversized_output_count += 1;
            self.oversized_post_op_return_bytes += post_op_return_bytes;
        }
    }
}

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_lengths = indexer.safe_lengths();
        let raw = &indexer.vecs.op_return;
        let txs = &indexer.vecs.transactions;
        let version = raw.first_index.version()
            + raw.to_tx_index.version()
            + raw.kind.version()
            + raw.post_op_return_bytes.version()
            + txs.weight.version();

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
                    }

                    total.post_op_return_bytes += bytes;
                    by_kind[kind_index].output_count += 1;
                    by_kind[kind_index].post_op_return_bytes += bytes;
                    carrier.add_output(kind, bytes);
                }

                finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

                self.total.push(total);
                for (kind, metrics) in self.by_kind.iter_typed_mut() {
                    metrics.push(by_kind[kind as usize]);
                }
                self.policy.standard.push(policy.standard);
                self.policy.oversized.push(policy.oversized);
                self.policy.multiple.push(policy.multiple);
                self.policy
                    .pre_v30_nonstandard
                    .push(policy.pre_v30_nonstandard);

                if (height + 1).is_multiple_of(WRITE_INTERVAL) {
                    let _lock = exit.lock();
                    self.write()?;
                }
                start = block_end;
            }

            let _lock = exit.lock();
            self.write()?;
        }

        self.compute_cumulative(starting_lengths.height, exit)?;

        let exit = exit.clone();
        self.db.run_bg(move |db| {
            let _lock = exit.lock();
            db.compact_deferred_default()
        });
        Ok(())
    }
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

    add_carrier(total, carrier.vsize);
    let mut kinds = carrier.kinds;
    while kinds != 0 {
        let kind_index = kinds.trailing_zeros() as usize;
        add_carrier(&mut by_kind[kind_index], carrier.vsize);
        kinds &= kinds - 1;
    }

    if carrier.oversized_output_count > 0 {
        policy.oversized.output_count += carrier.oversized_output_count;
        policy.oversized.post_op_return_bytes += carrier.oversized_post_op_return_bytes;
        add_carrier(&mut policy.oversized, carrier.vsize);
    }

    if carrier.output_count > 1 {
        policy.multiple.output_count += carrier.output_count;
        policy.multiple.post_op_return_bytes += carrier.post_op_return_bytes;
        add_carrier(&mut policy.multiple, carrier.vsize);
    }

    if carrier.oversized_output_count > 0 || carrier.output_count > 1 {
        policy.pre_v30_nonstandard.output_count += carrier.output_count;
        policy.pre_v30_nonstandard.post_op_return_bytes += carrier.post_op_return_bytes;
        add_carrier(&mut policy.pre_v30_nonstandard, carrier.vsize);
    } else {
        policy.standard.output_count += carrier.output_count;
        policy.standard.post_op_return_bytes += carrier.post_op_return_bytes;
        add_carrier(&mut policy.standard, carrier.vsize);
    }
}

fn add_carrier(metrics: &mut Totals, vsize: VSize) {
    metrics.carrier_tx_count += 1;
    metrics.carrier_vsize += vsize;
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
            ..Carrier::default()
        };
        carrier.add_output(OpReturnKind::Runes, 15);
        carrier.add_output(OpReturnKind::Omni, 15);

        finalize_transaction(&mut total, &mut by_kind, &mut policy, carrier);

        assert_eq!(total.carrier_tx_count, 1);
        assert_eq!(by_kind[OpReturnKind::Runes as usize].carrier_tx_count, 1);
        assert_eq!(by_kind[OpReturnKind::Omni as usize].carrier_tx_count, 1);
        assert_eq!(policy.multiple.output_count, 2);
        assert_eq!(policy.pre_v30_nonstandard.carrier_tx_count, 1);
        assert_eq!(policy.oversized.carrier_tx_count, 0);
        assert_eq!(policy.standard.carrier_tx_count, 0);
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
        assert_eq!(policy.oversized.carrier_vsize, VSize::new(120));
        assert_eq!(policy.pre_v30_nonstandard.carrier_tx_count, 1);
        assert_eq!(policy.multiple.carrier_tx_count, 0);
        assert_eq!(policy.standard.carrier_tx_count, 0);
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

        assert_eq!(policy.standard.output_count, 1);
        assert_eq!(policy.standard.post_op_return_bytes, 15);
        assert_eq!(policy.standard.carrier_tx_count, 1);
        assert_eq!(policy.standard.carrier_vsize, VSize::new(100));
        assert_eq!(policy.pre_v30_nonstandard.carrier_tx_count, 0);
    }
}
