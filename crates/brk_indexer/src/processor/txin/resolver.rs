use std::collections::hash_map::Entry;

use brk_error::{Error, Result};
use brk_types::{
    OutPoint, OutputType, SigOps, TxIndex, TxOutIndex, Txid, TxidPrefix, TypeIndex, Vout,
};
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
    reads: ReadBatch,
}

impl InputResolver {
    pub(crate) fn resolve(
        &mut self,
        processor: &BlockProcessor<'_>,
        txs: &[ComputedTx<'_>],
    ) -> Result<Vec<InputSource>> {
        self.prepare(txs, processor.lengths.tx_index);
        self.reads.resolve(
            processor,
            &self.previous_parents,
            &self.inputs,
            processor.lengths.tx_index,
        )?;

        let tracks_executed_legacy_sigops = processor.tracks_executed_legacy_sigops();
        let reads = &self.reads;

        self.inputs
            .par_iter()
            .enumerate()
            .map(|(input_index, input)| match *input {
                UnresolvedInput::Coinbase => Ok(InputSource::Coinbase),
                UnresolvedInput::SameBlock {
                    outpoint,
                    txout_offset,
                } => Ok(InputSource::SameBlock {
                    outpoint,
                    txout_offset,
                }),
                UnresolvedInput::PreviousBlock { parent_index, vout } => {
                    let parent = reads.parent(parent_index);
                    let outpoint = OutPoint::new(parent.tx_index, vout);
                    let (output_type, type_index) = reads.output(input_index);

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
struct ParentRead {
    original_index: usize,
    tx_index: TxIndex,
    first_txout_index: TxOutIndex,
}

#[derive(Clone, Copy)]
struct OutputRead {
    input_index: usize,
    txout_index: TxOutIndex,
}

#[derive(Default)]
struct ReadBatch {
    parents: Vec<ParentRead>,
    parent_positions: Vec<usize>,
    outputs: Vec<OutputRead>,
    output_positions: Vec<usize>,
    output_types: Vec<OutputType>,
    type_indices: Vec<TypeIndex>,
}

impl ReadBatch {
    fn resolve(
        &mut self,
        processor: &BlockProcessor<'_>,
        previous_parents: &[PreviousParent],
        inputs: &[UnresolvedInput],
        current_tx_index: TxIndex,
    ) -> Result<()> {
        self.resolve_parents(processor, previous_parents, current_tx_index)?;
        self.prepare_outputs(inputs);
        self.read_outputs(processor)
    }

    fn resolve_parents(
        &mut self,
        processor: &BlockProcessor<'_>,
        previous_parents: &[PreviousParent],
        current_tx_index: TxIndex,
    ) -> Result<()> {
        self.parents.clear();
        self.parents.extend(
            (0..previous_parents.len()).map(|original_index| ParentRead {
                original_index,
                tx_index: TxIndex::default(),
                first_txout_index: TxOutIndex::default(),
            }),
        );

        self.parents
            .par_iter_mut()
            .try_for_each(|read| {
                let parent = &previous_parents[read.original_index];
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

                read.tx_index = tx_index;
                Ok(())
            })?;

        self.parents.sort_unstable_by_key(|read| read.tx_index);
        self.parent_positions.clear();
        self.parent_positions.resize(self.parents.len(), 0);

        for (position, read) in self.parents.iter_mut().enumerate() {
            self.parent_positions[read.original_index] = position;
            read.first_txout_index = processor
                .vecs
                .transactions
                .first_txout_index
                .get_pushed_or_read(
                    read.tx_index,
                    &processor.readers.tx_index_to_first_txout_index,
                )
                .ok_or(Error::Internal("Missing txout_index"))?;
        }

        Ok(())
    }

    fn prepare_outputs(&mut self, inputs: &[UnresolvedInput]) {
        self.outputs.clear();
        self.outputs.reserve(inputs.len());

        for (input_index, input) in inputs.iter().enumerate() {
            if let UnresolvedInput::PreviousBlock { parent_index, vout } = *input {
                let parent = self.parent(parent_index);
                self.outputs.push(OutputRead {
                    input_index,
                    txout_index: parent.first_txout_index + vout,
                });
            }
        }

        self.outputs.sort_unstable_by_key(|read| read.txout_index);
        self.output_positions.clear();
        self.output_positions.resize(inputs.len(), 0);

        for (position, read) in self.outputs.iter().enumerate() {
            self.output_positions[read.input_index] = position;
        }
    }

    fn read_outputs(&mut self, processor: &BlockProcessor<'_>) -> Result<()> {
        self.output_types.clear();
        self.output_types.reserve(self.outputs.len());
        self.type_indices.clear();
        self.type_indices.reserve(self.outputs.len());

        let outputs = &self.outputs;
        let output_types = &mut self.output_types;
        let type_indices = &mut self.type_indices;

        let (output_types_result, type_indices_result) = rayon::join(
            || -> Result<()> {
                for read in outputs {
                    output_types.push(
                        processor
                            .vecs
                            .outputs
                            .output_type
                            .get_pushed_or_read(
                                read.txout_index,
                                &processor.readers.txout_index_to_output_type,
                            )
                            .ok_or(Error::Internal("Missing output_type"))?,
                    );
                }
                Ok(())
            },
            || -> Result<()> {
                for read in outputs {
                    type_indices.push(
                        processor
                            .vecs
                            .outputs
                            .type_index
                            .get_pushed_or_read(
                                read.txout_index,
                                &processor.readers.txout_index_to_type_index,
                            )
                            .ok_or(Error::Internal("Missing type_index"))?,
                    );
                }
                Ok(())
            },
        );

        output_types_result?;
        type_indices_result
    }

    fn parent(&self, original_index: usize) -> ParentRead {
        self.parents[self.parent_positions[original_index]]
    }

    fn output(&self, input_index: usize) -> (OutputType, TypeIndex) {
        let position = self.output_positions[input_index];
        (self.output_types[position], self.type_indices[position])
    }
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
