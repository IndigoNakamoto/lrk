use brk_error::Result;
use brk_indexer::{Indexer, Lengths};
use brk_types::{Height, StoredU64};
use vecdb::Exit;

use super::Vecs;
use crate::{
    inputs,
    internal::PerBlockCumulativeRolling,
    outputs::{ByTypeVecs, CountVecs},
};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        count: &CountVecs,
        inputs_count: &inputs::CountVecs,
        by_type: &ByTypeVecs,
        starting_lengths: &Lengths,
        indexer: &Indexer,
        exit: &Exit,
    ) -> Result<()> {
        let op_return: &PerBlockCumulativeRolling<StoredU64, StoredU64> =
            &by_type.output_count.by_type.unspendable.op_return;

        let bip30_dups = indexer.chain.constants().bip30_duplicate_heights;

        self.count.height.compute_transform3(
            starting_lengths.height,
            &count.total.cumulative.height,
            &inputs_count.cumulative.height,
            &op_return.cumulative.height,
            |(h, output_count, input_count, op_return_count, ..)| {
                let block_count = u64::from(h + 1_usize);
                // -1 > genesis output is unspendable
                let mut utxo_count =
                    *output_count - (*input_count - block_count) - *op_return_count - 1;

                // BIP30: subtract one UTXO for each duplicate coinbase that has
                // been seen at or before this height (chain-specific).
                for &(dup_height, _orig_height) in bip30_dups {
                    if h >= Height::new(dup_height) {
                        utxo_count -= 1;
                    }
                }

                (h, StoredU64::from(utxo_count))
            },
            exit,
        )?;
        Ok(())
    }
}
