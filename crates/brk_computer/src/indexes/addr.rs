use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{
    Addr, AddrBytes, EmptyOutputIndex, OpReturnIndex, P2AAddrIndex, P2ABytes, P2MSOutputIndex,
    P2PK33AddrIndex, P2PK33Bytes, P2PK65AddrIndex, P2PK65Bytes, P2PKHAddrIndex, P2PKHBytes,
    P2SHAddrIndex, P2SHBytes, P2TRAddrIndex, P2TRBytes, P2WPKHAddrIndex, P2WPKHBytes,
    P2WSHAddrIndex, P2WSHBytes, TxIndex, UnknownOutputIndex, Version,
};
use vecdb::{LazyVec, ReadableCloneableVec};

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub p2pk33: P2PK33Vecs,
    pub p2pk65: P2PK65Vecs,
    pub p2pkh: P2PKHVecs,
    pub p2sh: P2SHVecs,
    pub p2tr: P2TRVecs,
    pub p2wpkh: P2WPKHVecs,
    pub p2wsh: P2WSHVecs,
    pub p2a: P2AVecs,
    pub p2ms: P2MSVecs,
    pub empty: EmptyVecs,
    pub unknown: UnknownVecs,
    pub op_return: OpReturnVecs,
}

#[derive(Clone, Traversable)]
pub struct P2PK33Vecs {
    pub identity: LazyVec<P2PK33AddrIndex, P2PK33AddrIndex, P2PK33AddrIndex, P2PK33Bytes>,
    pub addr: LazyVec<P2PK33AddrIndex, Addr, P2PK33AddrIndex, P2PK33Bytes>,
}

#[derive(Clone, Traversable)]
pub struct P2PK65Vecs {
    pub identity: LazyVec<P2PK65AddrIndex, P2PK65AddrIndex, P2PK65AddrIndex, P2PK65Bytes>,
    pub addr: LazyVec<P2PK65AddrIndex, Addr, P2PK65AddrIndex, P2PK65Bytes>,
}

#[derive(Clone, Traversable)]
pub struct P2PKHVecs {
    pub identity: LazyVec<P2PKHAddrIndex, P2PKHAddrIndex, P2PKHAddrIndex, P2PKHBytes>,
    pub addr: LazyVec<P2PKHAddrIndex, Addr, P2PKHAddrIndex, P2PKHBytes>,
}

#[derive(Clone, Traversable)]
pub struct P2SHVecs {
    pub identity: LazyVec<P2SHAddrIndex, P2SHAddrIndex, P2SHAddrIndex, P2SHBytes>,
    pub addr: LazyVec<P2SHAddrIndex, Addr, P2SHAddrIndex, P2SHBytes>,
}

#[derive(Clone, Traversable)]
pub struct P2TRVecs {
    pub identity: LazyVec<P2TRAddrIndex, P2TRAddrIndex, P2TRAddrIndex, P2TRBytes>,
    pub addr: LazyVec<P2TRAddrIndex, Addr, P2TRAddrIndex, P2TRBytes>,
}

#[derive(Clone, Traversable)]
pub struct P2WPKHVecs {
    pub identity: LazyVec<P2WPKHAddrIndex, P2WPKHAddrIndex, P2WPKHAddrIndex, P2WPKHBytes>,
    pub addr: LazyVec<P2WPKHAddrIndex, Addr, P2WPKHAddrIndex, P2WPKHBytes>,
}

#[derive(Clone, Traversable)]
pub struct P2WSHVecs {
    pub identity: LazyVec<P2WSHAddrIndex, P2WSHAddrIndex, P2WSHAddrIndex, P2WSHBytes>,
    pub addr: LazyVec<P2WSHAddrIndex, Addr, P2WSHAddrIndex, P2WSHBytes>,
}

#[derive(Clone, Traversable)]
pub struct P2AVecs {
    pub identity: LazyVec<P2AAddrIndex, P2AAddrIndex, P2AAddrIndex, P2ABytes>,
    pub addr: LazyVec<P2AAddrIndex, Addr, P2AAddrIndex, P2ABytes>,
}

#[derive(Clone, Traversable)]
pub struct P2MSVecs {
    pub identity: LazyVec<P2MSOutputIndex, P2MSOutputIndex, P2MSOutputIndex, TxIndex>,
}

#[derive(Clone, Traversable)]
pub struct EmptyVecs {
    pub identity: LazyVec<EmptyOutputIndex, EmptyOutputIndex, EmptyOutputIndex, TxIndex>,
}

#[derive(Clone, Traversable)]
pub struct UnknownVecs {
    pub identity: LazyVec<UnknownOutputIndex, UnknownOutputIndex, UnknownOutputIndex, TxIndex>,
}

#[derive(Clone, Traversable)]
pub struct OpReturnVecs {
    pub identity: LazyVec<OpReturnIndex, OpReturnIndex, OpReturnIndex, TxIndex>,
}

impl Vecs {
    pub(crate) fn forced_import(version: Version, indexer: &Indexer) -> Self {
        Self {
            p2pk33: P2PK33Vecs {
                identity: LazyVec::init(
                    "p2pk33_addr_index",
                    version,
                    indexer.vecs().addrs.p2pk33.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2pk33_addr",
                    version,
                    indexer.vecs().addrs.p2pk33.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2pk65: P2PK65Vecs {
                identity: LazyVec::init(
                    "p2pk65_addr_index",
                    version,
                    indexer.vecs().addrs.p2pk65.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2pk65_addr",
                    version,
                    indexer.vecs().addrs.p2pk65.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2pkh: P2PKHVecs {
                identity: LazyVec::init(
                    "p2pkh_addr_index",
                    version,
                    indexer.vecs().addrs.p2pkh.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2pkh_addr",
                    version,
                    indexer.vecs().addrs.p2pkh.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2sh: P2SHVecs {
                identity: LazyVec::init(
                    "p2sh_addr_index",
                    version,
                    indexer.vecs().addrs.p2sh.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2sh_addr",
                    version,
                    indexer.vecs().addrs.p2sh.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2tr: P2TRVecs {
                identity: LazyVec::init(
                    "p2tr_addr_index",
                    version,
                    indexer.vecs().addrs.p2tr.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2tr_addr",
                    version,
                    indexer.vecs().addrs.p2tr.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2wpkh: P2WPKHVecs {
                identity: LazyVec::init(
                    "p2wpkh_addr_index",
                    version,
                    indexer.vecs().addrs.p2wpkh.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2wpkh_addr",
                    version,
                    indexer.vecs().addrs.p2wpkh.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2wsh: P2WSHVecs {
                identity: LazyVec::init(
                    "p2wsh_addr_index",
                    version,
                    indexer.vecs().addrs.p2wsh.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2wsh_addr",
                    version,
                    indexer.vecs().addrs.p2wsh.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2a: P2AVecs {
                identity: LazyVec::init(
                    "p2a_addr_index",
                    version,
                    indexer.vecs().addrs.p2a.bytes.read_only_boxed_clone(),
                    |index, _| index,
                ),
                addr: LazyVec::init(
                    "p2a_addr",
                    version,
                    indexer.vecs().addrs.p2a.bytes.read_only_boxed_clone(),
                    |_, bytes| Addr::try_from(&AddrBytes::from(bytes)).unwrap(),
                ),
            },
            p2ms: P2MSVecs {
                identity: LazyVec::init(
                    "p2ms_output_index",
                    version,
                    indexer
                        .vecs()
                        .scripts
                        .p2ms
                        .to_tx_index
                        .read_only_boxed_clone(),
                    |index, _| index,
                ),
            },
            empty: EmptyVecs {
                identity: LazyVec::init(
                    "empty_output_index",
                    version,
                    indexer
                        .vecs()
                        .scripts
                        .empty
                        .to_tx_index
                        .read_only_boxed_clone(),
                    |index, _| index,
                ),
            },
            unknown: UnknownVecs {
                identity: LazyVec::init(
                    "unknown_output_index",
                    version,
                    indexer
                        .vecs()
                        .scripts
                        .unknown
                        .to_tx_index
                        .read_only_boxed_clone(),
                    |index, _| index,
                ),
            },
            op_return: OpReturnVecs {
                identity: LazyVec::init(
                    "op_return_index",
                    version,
                    indexer.vecs().op_return.to_tx_index.read_only_boxed_clone(),
                    |index, _| index,
                ),
            },
        }
    }
}
