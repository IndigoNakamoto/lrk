use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{blocks, internal::ValuePerBlock, transactions};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        blocks: &blocks::Vecs,
        transactions: &transactions::Vecs,
        circulating_supply: &ValuePerBlock,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        // velocity = rolling_1y_sum(volume) / circulating_supply
        // `circulating_supply` = transparent UTXO supply + MWEB pegged balance.

        // Native velocity at height level
        self.native.height.compute_rolling_ratio(
            starting_height,
            &blocks.lookback._1y,
            &transactions.volume.transfer_volume.block.sats,
            &circulating_supply.sats.height,
            exit,
        )?;

        // Fiat velocity at height level
        self.fiat.height.compute_rolling_ratio(
            starting_height,
            &blocks.lookback._1y,
            &transactions.volume.transfer_volume.block.usd,
            &circulating_supply.usd.height,
            exit,
        )?;

        Ok(())
    }
}
