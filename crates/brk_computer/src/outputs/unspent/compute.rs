use brk_error::Result;
use brk_indexer::{Indexer, Lengths};
use brk_types::{Height, StoredU64};
use vecdb::Exit;

use super::Vecs;
use crate::{
    inputs,
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
        let op_return = &by_type.output_count.by_type.unspendable.op_return;

        let bip30_dups = indexer.chain().constants().bip30_duplicate_heights;

        // Note: unspendable Litecoin MWEB outputs are naturally kept out of the
        // spendable UTXO set here because every MWEB output that gets spent
        // (peg-ins consumed same-block, peg-pool re-spent each block) is
        // captured by `input_count`, leaving only the single currently-unspent
        // peg-pool output uncorrected — negligible in a UTXO-count estimate.
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
