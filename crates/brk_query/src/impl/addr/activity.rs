use brk_error::{Error, Result};
use brk_types::{Addr, Height, Txid};

use crate::Query;

impl Query {
    /// Height of the last on-chain activity for an address (last tx_index to height).
    /// With `before_txid`, returns the newest activity strictly older than that
    /// cursor. Used by paginated chain etags so a new tx above the cursor
    /// doesn't invalidate deeper pages.
    pub fn addr_last_activity_height(
        &self,
        addr: &Addr,
        before_txid: Option<&Txid>,
    ) -> Result<Height> {
        let (output_type, type_index) = self.resolve_addr(addr)?;
        let stores = self.indexer().stores();
        let tx_index_len = self.safe_lengths().tx_index;
        let last_tx_index = match before_txid {
            Some(txid) => {
                let before_tx_index = self.resolve_tx_index(txid)?;
                stores
                    .addr_tx_indexes_before(output_type, type_index, before_tx_index)?
                    .rev()
                    .find(|tx_index| *tx_index < tx_index_len)
                    .ok_or(Error::UnknownAddr)?
            }
            None => stores
                .addr_tx_indexes(output_type, type_index)?
                .rev()
                .find(|tx_index| *tx_index < tx_index_len)
                .ok_or(Error::UnknownAddr)?,
        };
        self.confirmed_status_height(last_tx_index)
    }
}
