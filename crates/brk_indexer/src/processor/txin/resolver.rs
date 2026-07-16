use std::collections::hash_map::Entry;

use brk_error::{Error, Result};
use brk_types::{OutPoint, SigOps, TxIndex, TxOutIndex, Txid, TxidPrefix, Vout};
use rayon::prelude::*;
use rustc_hash::FxHashMap;
use tracing::error;
use vecdb::unlikely;

use super::InputSource;
use crate::processor::{BlockProcessor, transaction::ComputedTx};

#[derive(Default)]
pub(crate) struct InputResolver {
    same_block_transactions: FxHashMap<TxidPrefix, TxIndex>,
    previous_parent_indexes: FxHashMap<TxidPrefix, usize>,
    previous_parents: Vec<PreviousParent>,
    inputs: Vec<UnresolvedInput>,
}

impl InputResolver {
    pub(crate) fn resolve(
        &mut self,
        processor: &BlockProcessor<'_>,
        txs: &[ComputedTx<'_>],
    ) -> Result<Vec<InputSource>> {
        self.prepare(txs, processor.lengths.tx_index);

        let current_tx_index = processor.lengths.tx_index;
        let parents = self
            .previous_parents
            .par_iter()
            .map(|parent| {
                let store_result = processor
                    .stores
                    .txid_prefix_to_tx_index
                    .get(&parent.txid_prefix)?
                    .map(|value| *value);

                let tx_index = match store_result {
                    Some(tx_index) if tx_index < current_tx_index => tx_index,
                    _ => {
                        error!(
                            "UnknownTxid: txid={}, prefix={:?}, store_result={:?}, current_tx_index={:?}",
                            parent.txid,
                            parent.txid_prefix,
                            store_result,
                            current_tx_index
                        );
                        return Err(Error::UnknownTxid);
                    }
                };

                let first_txout_index = processor
                    .vecs
                    .transactions
                    .first_txout_index
                    .get_pushed_or_read(
                        tx_index,
                        &processor.readers.tx_index_to_first_txout_index,
                    )
                    .ok_or(Error::Internal("Missing txout_index"))?;

                Ok(ResolvedParent {
                    tx_index,
                    first_txout_index,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        let tracks_executed_legacy_sigops = processor.tracks_executed_legacy_sigops();

        self.inputs
            .par_iter()
            .map(|input| match *input {
                UnresolvedInput::Coinbase => Ok(InputSource::Coinbase),
                UnresolvedInput::SameBlock {
                    outpoint,
                    txout_offset,
                } => Ok(InputSource::SameBlock {
                    outpoint,
                    txout_offset,
                }),
                UnresolvedInput::PreviousBlock { parent_index, vout } => {
                    let parent = parents[parent_index];
                    let txout_index = parent.first_txout_index + vout;
                    let outpoint = OutPoint::new(parent.tx_index, vout);

                    let output_type = processor
                        .vecs
                        .outputs
                        .output_type
                        .get_pushed_or_read(
                            txout_index,
                            &processor.readers.txout_index_to_output_type,
                        )
                        .ok_or(Error::Internal("Missing output_type"))?;

                    let type_index = processor
                        .vecs
                        .outputs
                        .type_index
                        .get_pushed_or_read(
                            txout_index,
                            &processor.readers.txout_index_to_type_index,
                        )
                        .ok_or(Error::Internal("Missing type_index"))?;

                    let legacy_sigops = if tracks_executed_legacy_sigops {
                        processor
                            .vecs
                            .scripts
                            .legacy_sigops(output_type, type_index, &processor.readers.scripts)
                            .ok_or(Error::Internal("Missing legacy_sigops"))?
                    } else {
                        SigOps::ZERO
                    };

                    Ok(InputSource::PreviousBlock {
                        outpoint,
                        output_type,
                        legacy_sigops,
                        type_index,
                    })
                }
            })
            .collect()
    }

    fn prepare(&mut self, txs: &[ComputedTx<'_>], block_first_tx_index: TxIndex) {
        self.same_block_transactions.clear();
        self.previous_parent_indexes.clear();
        self.previous_parents.clear();
        self.inputs.clear();

        self.same_block_transactions.reserve(txs.len());
        self.same_block_transactions
            .extend(txs.iter().map(|tx| (tx.txid_prefix(), tx.tx_index)));

        let total_inputs = txs.iter().map(|tx| tx.tx.input.len()).sum();
        self.inputs.reserve(total_inputs);

        for tx in txs {
            for txin in &tx.tx.input {
                let previous_output = &txin.previous_output;
                if unlikely(previous_output.is_null()) {
                    self.inputs.push(UnresolvedInput::Coinbase);
                    continue;
                }

                let txid = *<&Txid>::from(&previous_output.txid);
                let txid_prefix = TxidPrefix::from(&txid);
                let vout = Vout::from(previous_output.vout);

                if let Some(&tx_index) = self.same_block_transactions.get(&txid_prefix) {
                    let block_tx_index = usize::from(tx_index) - usize::from(block_first_tx_index);
                    self.inputs.push(UnresolvedInput::SameBlock {
                        outpoint: OutPoint::new(tx_index, vout),
                        txout_offset: txs[block_tx_index].txout_offset(vout),
                    });
                    continue;
                }

                let parent_index = match self.previous_parent_indexes.entry(txid_prefix) {
                    Entry::Occupied(entry) => *entry.get(),
                    Entry::Vacant(entry) => {
                        let parent_index = self.previous_parents.len();
                        entry.insert(parent_index);
                        self.previous_parents
                            .push(PreviousParent { txid, txid_prefix });
                        parent_index
                    }
                };

                self.inputs
                    .push(UnresolvedInput::PreviousBlock { parent_index, vout });
            }
        }
    }
}

#[derive(Clone, Copy)]
struct PreviousParent {
    txid: Txid,
    txid_prefix: TxidPrefix,
}

#[derive(Clone, Copy)]
struct ResolvedParent {
    tx_index: TxIndex,
    first_txout_index: TxOutIndex,
}

#[derive(Clone, Copy)]
enum UnresolvedInput {
    Coinbase,
    PreviousBlock {
        parent_index: usize,
        vout: Vout,
    },
    SameBlock {
        outpoint: OutPoint,
        txout_offset: usize,
    },
}
