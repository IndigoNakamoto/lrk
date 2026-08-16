use brk_chain::primitives as bitcoin;
use bitcoin::Transaction;
use brk_types::{TxIndex, Txid, TxidPrefix, Vout};

pub(crate) struct ComputedTx<'a> {
    pub(crate) tx_index: TxIndex,
    pub(crate) tx: &'a Transaction,
    pub(crate) txid: Txid,
    pub(crate) insert_txid_prefix: bool,
    pub(crate) base_size: u32,
    pub(crate) total_size: u32,
    input_offset: usize,
    output_offset: usize,
}

impl ComputedTx<'_> {
    pub(super) fn new(
        tx_index: TxIndex,
        tx: &Transaction,
        txid: Txid,
        insert_txid_prefix: bool,
        base_size: u32,
        total_size: u32,
    ) -> ComputedTx<'_> {
        ComputedTx {
            tx_index,
            tx,
            txid,
            insert_txid_prefix,
            base_size,
            total_size,
            input_offset: 0,
            output_offset: 0,
        }
    }

    pub(super) fn set_block_offsets(txs: &mut [Self]) {
        let mut input_offset = 0;
        let mut output_offset = 0;

        for tx in txs {
            tx.input_offset = input_offset;
            tx.output_offset = output_offset;
            input_offset += tx.tx.input.len();
            output_offset += tx.tx.output.len();
        }
    }

    #[inline]
    pub(super) fn inputs<'a, T>(&self, inputs: &'a [T]) -> &'a [T] {
        &inputs[self.input_offset..self.input_offset + self.tx.input.len()]
    }

    #[inline]
    pub(super) fn outputs<'a, T>(&self, outputs: &'a [T]) -> &'a [T] {
        &outputs[self.output_offset..self.output_offset + self.tx.output.len()]
    }

    #[inline]
    pub(crate) fn txout_offset(&self, vout: Vout) -> usize {
        self.output_offset + usize::from(vout)
    }

    #[inline]
    pub(crate) fn is_coinbase(&self, block_first_tx_index: TxIndex) -> bool {
        self.tx_index == block_first_tx_index
    }

    #[inline]
    pub(crate) fn txid_prefix(&self) -> TxidPrefix {
        TxidPrefix::from(&self.txid)
    }

    #[inline]
    pub(crate) fn is_segwit(&self) -> bool {
        self.base_size != self.total_size
    }

    #[inline]
    pub(crate) fn weight(&self) -> bitcoin::Weight {
        brk_types::Weight::from_sizes(self.base_size, self.total_size).into()
    }
}
