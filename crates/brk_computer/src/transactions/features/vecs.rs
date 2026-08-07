use brk_traversable::Traversable;
use brk_types::StoredU64;
use vecdb::{Rw, StorageMode};

use crate::internal::PerBlockCumulativeRolling;

/// Transaction counts by detected feature.
///
/// Each metric counts transactions containing the feature at least once, not
/// individual occurrences. A transaction can contribute to multiple metrics.
#[derive(Traversable)]
pub struct CountVecs<M: StorageMode = Rw> {
    pub inscription: PerBlockCumulativeRolling<StoredU64, M>,
    pub annex: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions containing at least one `SIGHASH_ALL` signature.
    pub sighash_all: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions containing at least one `SIGHASH_NONE` signature.
    pub sighash_none: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions containing at least one `SIGHASH_SINGLE` signature.
    pub sighash_single: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions containing at least one Taproot `SIGHASH_DEFAULT` signature.
    pub sighash_default: PerBlockCumulativeRolling<StoredU64, M>,
    /// Transactions containing at least one `SIGHASH_ANYONECANPAY` signature.
    ///
    /// This modifier is counted independently from ALL, NONE, and SINGLE.
    pub sighash_anyone_can_pay: PerBlockCumulativeRolling<StoredU64, M>,
    pub dust_output: PerBlockCumulativeRolling<StoredU64, M>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub count: CountVecs<M>,
}
