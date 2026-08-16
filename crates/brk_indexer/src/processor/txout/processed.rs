use brk_types::{AddrHash, OutputType, SigOps, TypeIndex};

use super::op_return;

#[derive(Clone, Copy)]
pub(super) enum ProcessedOutputData {
    Address(AddrHash),
    OpReturn(op_return::Facts),
    None,
    Resolved(TypeIndex),
}

#[derive(Clone, Copy)]
pub(crate) struct ProcessedOutput {
    pub(crate) output_type: OutputType,
    pub(crate) legacy_sigops: SigOps,
    pub(super) data: ProcessedOutputData,
}

impl ProcessedOutput {
    pub(crate) fn op_return_legacy_sigops(&self) -> usize {
        let ProcessedOutputData::OpReturn(facts) = self.data else {
            unreachable!();
        };
        u32::from(facts.legacy_sigops) as usize
    }

    pub(crate) fn resolved_type_index(&self) -> TypeIndex {
        let ProcessedOutputData::Resolved(type_index) = self.data else {
            unreachable!();
        };
        type_index
    }
}
