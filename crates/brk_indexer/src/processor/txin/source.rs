use brk_types::{OutPoint, OutputType, SigOps, TypeIndex};

#[derive(Debug)]
pub(crate) enum InputSource {
    Coinbase,
    PreviousBlock {
        outpoint: OutPoint,
        output_type: OutputType,
        legacy_sigops: SigOps,
        type_index: TypeIndex,
    },
    SameBlock {
        outpoint: OutPoint,
        txout_offset: usize,
    },
}
