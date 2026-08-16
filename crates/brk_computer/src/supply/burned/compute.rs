use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::Sats;
use vecdb::{Exit, VecIndex};

use super::Vecs;
use crate::{mining, outputs, price};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        outputs: &outputs::Vecs,
        mining: &mining::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        self.total.compute_from_pair(
            starting_height,
            prices,
            &outputs.value.op_return.block.sats,
            &mining.rewards.unclaimed.block.sats,
            |height, op_return, unclaimed| {
                let genesis = if height.to_usize() == 0 {
                    Sats::FIFTY_BTC
                } else {
                    Sats::ZERO
                };
                genesis + op_return + unclaimed
            },
            exit,
        )?;
        Ok(())
    }
}
