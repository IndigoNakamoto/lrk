use brk_indexer::Indexer;
use brk_types::{
    AnyAddrIndex, EmptyAddrData, EmptyAddrIndex, FundedAddrData, FundedAddrIndex, Height, OutPoint,
    OutputType, P2AAddrIndex, P2PK33AddrIndex, P2PK65AddrIndex, P2PKHAddrIndex, P2SHAddrIndex,
    P2TRAddrIndex, P2WPKHAddrIndex, P2WSHAddrIndex, Sats, StoredU64, TxInIndex, TxIndex, TypeIndex,
};
use vecdb::{BytesVecReader, PcoVec, ReadableVec, VecIndex};

use crate::distribution::{
    RangeMap,
    addr::{AddrsDataVecs, AnyAddrIndexesVecs},
};

/// Output data collected from separate vecs.
#[derive(Debug, Clone, Copy)]
pub struct TxOutData {
    pub value: Sats,
    pub output_type: OutputType,
    pub type_index: TypeIndex,
}

/// Readers for txout vectors. Reuses internal buffers across blocks.
pub struct TxOutReaders<'a> {
    indexer: &'a Indexer,
    values_buf: Vec<Sats>,
    output_types_buf: Vec<OutputType>,
    type_indexes_buf: Vec<TypeIndex>,
    txout_data_buf: Vec<TxOutData>,
}

impl<'a> TxOutReaders<'a> {
    pub(crate) fn new(indexer: &'a Indexer) -> Self {
        Self {
            indexer,
            values_buf: Vec::new(),
            output_types_buf: Vec::new(),
            type_indexes_buf: Vec::new(),
            txout_data_buf: Vec::new(),
        }
    }

    /// Collect output data for a block range using bulk reads with buffer reuse.
    pub(crate) fn collect_block_outputs(
        &mut self,
        first_txout_index: usize,
        output_count: usize,
    ) -> &[TxOutData] {
        let end = first_txout_index + output_count;
        self.indexer.vecs().outputs.value.collect_range_into_at(
            first_txout_index,
            end,
            &mut self.values_buf,
        );
        self.indexer
            .vecs()
            .outputs
            .output_type
            .collect_range_into_at(first_txout_index, end, &mut self.output_types_buf);
        self.indexer
            .vecs()
            .outputs
            .type_index
            .collect_range_into_at(first_txout_index, end, &mut self.type_indexes_buf);

        self.txout_data_buf.clear();
        self.txout_data_buf.extend(
            self.values_buf
                .iter()
                .zip(&self.output_types_buf)
                .zip(&self.type_indexes_buf)
                .map(|((&value, &output_type), &type_index)| TxOutData {
                    value,
                    output_type,
                    type_index,
                }),
        );
        &self.txout_data_buf
    }
}

/// Readers for txin vectors. Reuses all buffers across blocks.
pub struct TxInReaders<'a> {
    indexer: &'a Indexer,
    input_values: &'a PcoVec<TxInIndex, Sats>,
    tx_index_to_height: &'a mut RangeMap<TxIndex, Height>,
    outpoints_buf: Vec<OutPoint>,
    values_buf: Vec<Sats>,
    prev_heights_buf: Vec<Height>,
    output_types_buf: Vec<OutputType>,
    type_indexes_buf: Vec<TypeIndex>,
}

impl<'a> TxInReaders<'a> {
    pub(crate) fn new(
        indexer: &'a Indexer,
        input_values: &'a PcoVec<TxInIndex, Sats>,
        tx_index_to_height: &'a mut RangeMap<TxIndex, Height>,
    ) -> Self {
        Self {
            indexer,
            input_values,
            tx_index_to_height,
            outpoints_buf: Vec::new(),
            values_buf: Vec::new(),
            prev_heights_buf: Vec::new(),
            output_types_buf: Vec::new(),
            type_indexes_buf: Vec::new(),
        }
    }

    /// Collect input data for a block range using bulk reads with buffer reuse.
    pub(crate) fn collect_block_inputs(
        &mut self,
        first_txin_index: usize,
        input_count: usize,
        current_height: Height,
    ) -> (&[Sats], &[Height], &[OutputType], &[TypeIndex]) {
        let end = first_txin_index + input_count;
        self.input_values
            .collect_range_into_at(first_txin_index, end, &mut self.values_buf);
        self.indexer.vecs().inputs.outpoint.collect_range_into_at(
            first_txin_index,
            end,
            &mut self.outpoints_buf,
        );
        self.indexer
            .vecs()
            .inputs
            .output_type
            .collect_range_into_at(first_txin_index, end, &mut self.output_types_buf);
        self.indexer.vecs().inputs.type_index.collect_range_into_at(
            first_txin_index,
            end,
            &mut self.type_indexes_buf,
        );

        self.prev_heights_buf.clear();
        self.prev_heights_buf
            .extend(self.outpoints_buf.iter().map(|outpoint| {
                if outpoint.is_coinbase() {
                    current_height
                } else {
                    self.tx_index_to_height
                        .get(outpoint.tx_index())
                        .unwrap_or(current_height)
                }
            }));

        (
            &self.values_buf,
            &self.prev_heights_buf,
            &self.output_types_buf,
            &self.type_indexes_buf,
        )
    }
}

/// Cached readers for stateful vectors.
pub struct VecsReaders {
    p2a: BytesVecReader<P2AAddrIndex, AnyAddrIndex>,
    p2pk33: BytesVecReader<P2PK33AddrIndex, AnyAddrIndex>,
    p2pk65: BytesVecReader<P2PK65AddrIndex, AnyAddrIndex>,
    p2pkh: BytesVecReader<P2PKHAddrIndex, AnyAddrIndex>,
    p2sh: BytesVecReader<P2SHAddrIndex, AnyAddrIndex>,
    p2tr: BytesVecReader<P2TRAddrIndex, AnyAddrIndex>,
    p2wpkh: BytesVecReader<P2WPKHAddrIndex, AnyAddrIndex>,
    p2wsh: BytesVecReader<P2WSHAddrIndex, AnyAddrIndex>,
    funded: BytesVecReader<FundedAddrIndex, FundedAddrData>,
    empty: BytesVecReader<EmptyAddrIndex, EmptyAddrData>,
}

impl VecsReaders {
    pub(crate) fn new(any_addr_indexes: &AnyAddrIndexesVecs, addrs_data: &AddrsDataVecs) -> Self {
        Self {
            p2a: any_addr_indexes.p2a.reader(),
            p2pk33: any_addr_indexes.p2pk33.reader(),
            p2pk65: any_addr_indexes.p2pk65.reader(),
            p2pkh: any_addr_indexes.p2pkh.reader(),
            p2sh: any_addr_indexes.p2sh.reader(),
            p2tr: any_addr_indexes.p2tr.reader(),
            p2wpkh: any_addr_indexes.p2wpkh.reader(),
            p2wsh: any_addr_indexes.p2wsh.reader(),
            funded: addrs_data.funded.reader(),
            empty: addrs_data.empty.reader(),
        }
    }

    /// Read the unified address index, including uncommitted updates after rollback.
    pub(crate) fn any_addr_index(
        &self,
        vecs: &AnyAddrIndexesVecs,
        addr_type: OutputType,
        type_index: TypeIndex,
    ) -> AnyAddrIndex {
        let index = match addr_type {
            OutputType::P2A => vecs.p2a.get_with_reader(type_index.into(), &self.p2a),
            OutputType::P2PK33 => vecs.p2pk33.get_with_reader(type_index.into(), &self.p2pk33),
            OutputType::P2PK65 => vecs.p2pk65.get_with_reader(type_index.into(), &self.p2pk65),
            OutputType::P2PKH => vecs.p2pkh.get_with_reader(type_index.into(), &self.p2pkh),
            OutputType::P2SH => vecs.p2sh.get_with_reader(type_index.into(), &self.p2sh),
            OutputType::P2TR => vecs.p2tr.get_with_reader(type_index.into(), &self.p2tr),
            OutputType::P2WPKH => vecs.p2wpkh.get_with_reader(type_index.into(), &self.p2wpkh),
            OutputType::P2WSH => vecs.p2wsh.get_with_reader(type_index.into(), &self.p2wsh),
            _ => unreachable!("invalid address type: {addr_type:?}"),
        };
        index.unwrap()
    }

    #[inline]
    pub(crate) fn funded_data(
        &self,
        vecs: &AddrsDataVecs,
        index: FundedAddrIndex,
    ) -> FundedAddrData {
        vecs.funded.get_with_reader(index, &self.funded).unwrap()
    }

    #[inline]
    pub(crate) fn empty_data(&self, vecs: &AddrsDataVecs, index: EmptyAddrIndex) -> EmptyAddrData {
        vecs.empty.get_with_reader(index, &self.empty).unwrap()
    }
}

/// Reusable buffers for per-block tx_index mapping construction.
pub(crate) struct IndexToTxIndexBuf {
    counts: Vec<StoredU64>,
    result: Vec<TxIndex>,
}

impl IndexToTxIndexBuf {
    pub(crate) fn new() -> Self {
        Self {
            counts: Vec::new(),
            result: Vec::new(),
        }
    }

    /// Build index -> tx_index mapping for a block, reusing internal buffers.
    pub(crate) fn build(
        &mut self,
        block_first_tx_index: TxIndex,
        block_tx_count: u64,
        tx_index_to_count: &impl ReadableVec<TxIndex, StoredU64>,
    ) -> &[TxIndex] {
        let first = block_first_tx_index.to_usize();
        tx_index_to_count.collect_range_into_at(
            first,
            first + block_tx_count as usize,
            &mut self.counts,
        );

        let total: u64 = self.counts.iter().map(|c| u64::from(*c)).sum();
        self.result.clear();
        self.result.reserve(total as usize);

        for (offset, count) in self.counts.iter().enumerate() {
            let tx_index = TxIndex::from(first + offset);
            self.result
                .extend(std::iter::repeat_n(tx_index, u64::from(*count) as usize));
        }

        &self.result
    }
}
