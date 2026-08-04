use brk_types::{OutPoint, OutputType, Sats, SigOps, TxOutIndex, TypeIndex};

#[derive(Debug, Clone, Copy)]
pub(crate) enum InputSource {
    Coinbase,
    PreviousBlock {
        outpoint: OutPoint,
        txout_index: TxOutIndex,
        value: Sats,
        output_type: OutputType,
        legacy_sigops: SigOps,
        type_index: TypeIndex,
    },
    SameBlock {
        outpoint: OutPoint,
        txout_offset: usize,
        txout_index: TxOutIndex,
        value: Sats,
    },
}
