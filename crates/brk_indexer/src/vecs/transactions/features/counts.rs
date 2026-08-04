use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, StoredU64, TxVersion, Version};
use rayon::prelude::*;
use vecdb::{AnyStoredVec, Database, ImportableVec, PcoVec, Rw, Stamp, StorageMode, WritableVec};

use super::schema::with_transaction_features;

macro_rules! define_counts {
    ($($(#[$attribute:meta])* $vector:ident: $flag:ident = $bit:literal $(, count: $count:ident)?;)+) => {
        #[derive(Default)]
        pub(crate) struct TransactionCounts {
            v1: u64,
            v2: u64,
            v3: u64,
            other_version: u64,
            explicitly_rbf: u64,
            one_input: u64,
            one_output: u64,
            $($(pub(super) $count: u64,)?) +
        }

        impl TransactionCounts {
            pub(crate) fn add_base(
                &mut self,
                input_count: usize,
                output_count: usize,
                version: TxVersion,
                explicitly_rbf: bool,
            ) {
                match version {
                    TxVersion::ONE => self.v1 += 1,
                    TxVersion::TWO => self.v2 += 1,
                    TxVersion::THREE => self.v3 += 1,
                    _ => self.other_version += 1,
                }
                self.explicitly_rbf += explicitly_rbf as u64;
                self.one_input += (input_count == 1) as u64;
                self.one_output += (output_count == 1) as u64;
            }
        }

        #[derive(Traversable)]
        pub struct TransactionCountVecs<M: StorageMode = Rw> {
            pub v1: M::Stored<PcoVec<Height, StoredU64>>,
            pub v2: M::Stored<PcoVec<Height, StoredU64>>,
            pub v3: M::Stored<PcoVec<Height, StoredU64>>,
            pub other_version: M::Stored<PcoVec<Height, StoredU64>>,
            pub explicitly_rbf: M::Stored<PcoVec<Height, StoredU64>>,
            pub one_input: M::Stored<PcoVec<Height, StoredU64>>,
            pub one_output: M::Stored<PcoVec<Height, StoredU64>>,
            $($(pub $count: M::Stored<PcoVec<Height, StoredU64>>,)?) +
        }

        impl TransactionCountVecs {
            pub fn forced_import(db: &Database, version: Version) -> Result<Self> {
                let (
                    v1,
                    v2,
                    v3,
                    other_version,
                    explicitly_rbf,
                    one_input,
                    one_output,
                    $($($count,)?) +
                ) = crate::parallel_import! {
                    v1 = PcoVec::forced_import(db, "tx_count_v1", version),
                    v2 = PcoVec::forced_import(db, "tx_count_v2", version),
                    v3 = PcoVec::forced_import(db, "tx_count_v3", version),
                    other_version = PcoVec::forced_import(db, "tx_count_other_version", version),
                    explicitly_rbf = PcoVec::forced_import(db, "tx_count_explicitly_rbf", version),
                    one_input = PcoVec::forced_import(db, "tx_count_one_input", version),
                    one_output = PcoVec::forced_import(db, "tx_count_one_output", version),
                    $($($count = PcoVec::forced_import(
                        db,
                        concat!("tx_count_", stringify!($count)),
                        version,
                    ),)?) +
                };
                Ok(Self {
                    v1,
                    v2,
                    v3,
                    other_version,
                    explicitly_rbf,
                    one_input,
                    one_output,
                    $($($count,)?) +
                })
            }

            pub(crate) fn push(&mut self, height: Height, counts: TransactionCounts) {
                self.v1.debug_checked_push(height, counts.v1.into());
                self.v2.debug_checked_push(height, counts.v2.into());
                self.v3.debug_checked_push(height, counts.v3.into());
                self.other_version
                    .debug_checked_push(height, counts.other_version.into());
                self.explicitly_rbf
                    .debug_checked_push(height, counts.explicitly_rbf.into());
                self.one_input.debug_checked_push(height, counts.one_input.into());
                self.one_output.debug_checked_push(height, counts.one_output.into());
                $($(self.$count.debug_checked_push(height, counts.$count.into());)?) +
            }

            pub(crate) fn truncate(&mut self, height: Height, stamp: Stamp) -> Result<()> {
                self.v1.truncate_if_needed_with_stamp(height, stamp)?;
                self.v2.truncate_if_needed_with_stamp(height, stamp)?;
                self.v3.truncate_if_needed_with_stamp(height, stamp)?;
                self.other_version
                    .truncate_if_needed_with_stamp(height, stamp)?;
                self.explicitly_rbf
                    .truncate_if_needed_with_stamp(height, stamp)?;
                self.one_input.truncate_if_needed_with_stamp(height, stamp)?;
                self.one_output.truncate_if_needed_with_stamp(height, stamp)?;
                $($(self.$count.truncate_if_needed_with_stamp(height, stamp)?;)?) +
                Ok(())
            }

            pub(crate) fn par_iter_mut_any(
                &mut self,
            ) -> impl ParallelIterator<Item = &mut dyn AnyStoredVec> {
                [
                    &mut self.v1 as &mut dyn AnyStoredVec,
                    &mut self.v2,
                    &mut self.v3,
                    &mut self.other_version,
                    &mut self.explicitly_rbf,
                    &mut self.one_input,
                    &mut self.one_output,
                    $($(&mut self.$count,)?) +
                ]
                .into_par_iter()
            }

            pub(crate) fn iter_any(&self) -> impl Iterator<Item = &dyn AnyStoredVec> {
                [
                    &self.v1 as &dyn AnyStoredVec,
                    &self.v2,
                    &self.v3,
                    &self.other_version,
                    &self.explicitly_rbf,
                    &self.one_input,
                    &self.one_output,
                    $($(&self.$count,)?) +
                ]
                .into_iter()
            }
        }
    };
}

with_transaction_features!(define_counts);

#[cfg(test)]
mod tests {
    use brk_types::TxVersion;

    use super::TransactionCounts;

    #[test]
    fn counts_base_transaction_properties() {
        let mut counts = TransactionCounts::default();
        counts.add_base(1, 2, TxVersion::TWO, true);
        counts.add_base(2, 1, TxVersion::NON_STANDARD, false);

        assert_eq!(counts.v1, 0);
        assert_eq!(counts.v2, 1);
        assert_eq!(counts.v3, 0);
        assert_eq!(counts.other_version, 1);
        assert_eq!(counts.explicitly_rbf, 1);
        assert_eq!(counts.one_input, 1);
        assert_eq!(counts.one_output, 1);
    }
}
