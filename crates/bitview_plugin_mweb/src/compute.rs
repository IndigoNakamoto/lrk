use bitview_compute::{prepare_computed, write_locked};
use bitview_plugin::{ComputePlugin, UpdateContext, compute_items};
use bitview_plugin_indexer::Indexer;
use bitview_primitives::{Boolean, Count32};
use brk_error::{OptionData, Result};
use brk_exit::Exit;
use bitview_primitives::TxOutIndex;
use brk_types::{
    CheckedSub, Height, OutputType, Sats, SatsSigned, TxIndex, Version,
    litecoin::MWEB_ACTIVATION_HEIGHT,
};
use vecdb::{AnyStoredVec, AnyVec, ReadableVec, VecIndex, WritableVec};

use crate::{Dependencies, Vecs};

/// One block's HogEx, read from its inputs and outputs.
#[derive(Debug, Default, PartialEq)]
struct HogEx {
    /// The new HogAddr: output 0.
    supply: Sats,
    /// The previous HogAddr it spends, absent at activation.
    previous_supply: Option<Sats>,
    peg_in: Sats,
    peg_out: Sats,
    fee: Sats,
    peg_ins: u32,
    peg_outs: u32,
}

impl HogEx {
    fn read(inputs: &[(Sats, OutputType)], outputs: &[(Sats, OutputType)]) -> Option<Self> {
        let (&(supply, OutputType::MWEBPegPool), peg_outs) = outputs.split_first()? else {
            return None;
        };
        let mut this = Self {
            supply,
            ..Self::default()
        };
        for &(value, output_type) in inputs {
            match output_type {
                OutputType::MWEBPegPool => {
                    *this.previous_supply.get_or_insert(Sats::ZERO) += value;
                }
                _ => {
                    this.peg_in += value;
                    this.peg_ins += 1;
                }
            }
        }
        for &(value, _) in peg_outs {
            this.peg_out += value;
        }
        this.peg_outs = peg_outs.len() as u32;
        let spent = this.previous_supply.unwrap_or(Sats::ZERO) + this.peg_in;
        this.fee = spent
            .checked_sub(supply + this.peg_out)
            .unwrap_or(Sats::ZERO);
        Some(this)
    }
}

fn signed(sats: Sats) -> i64 {
    u64::from(sats) as i64
}

impl Vecs {
    fn stored_mut(&mut self) -> [&mut dyn AnyStoredVec; 10] {
        [
            &mut self.supply_sats.height,
            &mut self.peg_in.cumulative.sats.height,
            &mut self.peg_out.cumulative.sats.height,
            &mut self.fee.cumulative.sats.height,
            &mut self.net_flow.height,
            &mut self.pegin_count.height,
            &mut self.pegout_count.height,
            &mut self.active.height,
            &mut self.accounting_mismatch.height,
            &mut self.link_mismatch.height,
        ]
    }

    fn compute_blocks(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let v = indexer.vecs();
        let version = [
            v.transactions.first_tx_index.version(),
            v.transactions.first_txin_index.version(),
            v.transactions.first_txout_index.version(),
            v.inputs.value.version(),
            v.inputs.output_type.version(),
            v.outputs.value.version(),
            v.outputs.output_type.version(),
            v.mweb.kernel_fee.version(),
        ]
        .into_iter()
        .sum::<Version>();
        let start = prepare_computed(
            self.stored_mut(),
            version,
            indexer.compute_start().height.to_usize(),
            exit,
        )?;
        let end = v.blocks.timestamp.len();
        if start >= end {
            return Ok(());
        }

        let total_txs = v.transactions.txid.len();
        let total_txins = v.inputs.output_type.len();
        let total_txouts = v.outputs.value.len();
        let next = |vec: &dyn Fn(usize) -> Option<usize>, h: usize, total: usize| {
            if h + 1 < end { vec(h + 1).unwrap_or(total) } else { total }
        };

        let mut supply = match start.checked_sub(1) {
            Some(prev) => self.supply_sats.height.collect_one_at(prev).data()?,
            None => Sats::ZERO,
        };
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();
        for h in start..end {
            let hogex = (h >= MWEB_ACTIVATION_HEIGHT.to_usize())
                .then(|| -> Option<HogEx> {
                    let tx_end = next(
                        &|i| v.transactions.first_tx_index.collect_one_at(i).map(usize::from),
                        h,
                        total_txs,
                    );
                    let last = TxIndex::from(tx_end.checked_sub(1)?);
                    let txin_start = usize::from(v.transactions.first_txin_index.collect_one_at(last.to_usize())?);
                    let txin_end = next(
                        &|i| v.inputs.first_txin_index.collect_one_at(i).map(usize::from),
                        h,
                        total_txins,
                    );
                    let txout_start: TxOutIndex =
                        v.transactions.first_txout_index.collect_one_at(last.to_usize())?.into();
                    let txout_end = next(
                        &|i| v.outputs.first_txout_index.collect_one_at(i).map(usize::from),
                        h,
                        total_txouts,
                    );
                    inputs.clear();
                    inputs.extend(
                        v.inputs
                            .value
                            .collect_range_at(txin_start, txin_end)
                            .into_iter()
                            .zip(v.inputs.output_type.collect_range_at(txin_start, txin_end)),
                    );
                    outputs.clear();
                    let txout_start = usize::from(txout_start);
                    outputs.extend(
                        v.outputs
                            .value
                            .collect_range_at(txout_start, txout_end)
                            .into_iter()
                            .zip(v.outputs.output_type.collect_range_at(txout_start, txout_end)),
                    );
                    HogEx::read(&inputs, &outputs)
                })
                .flatten();

            let height = Height::from(h);
            let previous = supply;
            match &hogex {
                Some(hogex) => {
                    supply = hogex.supply;
                    let kernels = signed(v.mweb.kernel_pegin.collect_one_at(h).data()?)
                        - signed(v.mweb.kernel_pegout.collect_one_at(h).data()?)
                        - signed(v.mweb.kernel_fee.collect_one_at(h).data()?);
                    let mismatch = signed(supply) - signed(previous) - kernels;
                    let link = signed(hogex.previous_supply.unwrap_or(Sats::ZERO)) - signed(previous);
                    self.peg_in.push_sats(hogex.peg_in);
                    self.peg_out.push_sats(hogex.peg_out);
                    self.fee.push_sats(hogex.fee);
                    self.net_flow.height.push(SatsSigned::from(
                        signed(hogex.peg_in) - signed(hogex.peg_out),
                    ));
                    self.pegin_count.height.push(Count32::from(hogex.peg_ins));
                    self.pegout_count.height.push(Count32::from(hogex.peg_outs));
                    self.active.height.push(Boolean::from(true));
                    self.accounting_mismatch.height.push(SatsSigned::from(mismatch));
                    self.link_mismatch.height.push(SatsSigned::from(link));
                }
                None => {
                    self.peg_in.push_sats(Sats::ZERO);
                    self.peg_out.push_sats(Sats::ZERO);
                    self.fee.push_sats(Sats::ZERO);
                    self.net_flow.height.push(SatsSigned::from(0i64));
                    self.pegin_count.height.push(Count32::from(0u32));
                    self.pegout_count.height.push(Count32::from(0u32));
                    self.active.height.push(Boolean::from(false));
                    self.accounting_mismatch.height.push(SatsSigned::from(0i64));
                    self.link_mismatch.height.push(SatsSigned::from(0i64));
                }
            }
            debug_assert_eq!(self.supply_sats.height.len(), height.to_usize());
            self.supply_sats.height.push(supply);
        }
        Ok(())
    }
}

impl ComputePlugin for Vecs {
    compute_items!(Dependencies, db);

    fn compute_state(
        &mut self,
        dependencies: Self::Dependencies<'_>,
        context: UpdateContext<'_>,
    ) -> Result<()> {
        let Dependencies { indexer, price } = dependencies;
        let exit = context.exit();
        self.compute_blocks(indexer, exit)?;
        write_locked(self.stored_mut(), exit)?;

        let from = indexer.compute_start().height;
        let prices = &price.spot.cents.height;
        self.peg_in.compute_cents(from, prices, exit)?;
        self.peg_out.compute_cents(from, prices, exit)?;
        self.fee.compute_cents(from, prices, exit)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const POOL: OutputType = OutputType::MWEBPegPool;
    const PEGIN: OutputType = OutputType::MWEBPegIn;

    fn sats(n: u64) -> Sats {
        Sats::from(n)
    }

    #[test]
    fn activation_hogex_has_no_previous_pool() {
        let hogex = HogEx::read(&[(sats(500), PEGIN), (sats(300), PEGIN)], &[(sats(790), POOL)]).unwrap();
        assert_eq!(hogex.previous_supply, None);
        assert_eq!((hogex.peg_in, hogex.peg_ins, hogex.fee), (sats(800), 2, sats(10)));
    }

    #[test]
    fn pool_moves_by_pegs_and_fee() {
        let inputs = [(sats(1_000), POOL), (sats(200), PEGIN)];
        let outputs = [
            (sats(1_050), POOL),
            (sats(100), OutputType::P2WPKH),
            (sats(40), OutputType::P2PKH),
        ];
        let hogex = HogEx::read(&inputs, &outputs).unwrap();
        assert_eq!(hogex.previous_supply, Some(sats(1_000)));
        assert_eq!((hogex.peg_out, hogex.peg_outs), (sats(140), 2));
        assert_eq!(hogex.fee, sats(10));
        assert_eq!(
            signed(hogex.supply) - 1_000,
            signed(hogex.peg_in) - signed(hogex.peg_out) - signed(hogex.fee)
        );
    }

    #[test]
    fn not_a_hogex_without_the_pool_output() {
        assert_eq!(HogEx::read(&[(sats(1), PEGIN)], &[(sats(1), OutputType::P2WPKH)]), None);
        assert_eq!(HogEx::read(&[], &[]), None);
    }
}
