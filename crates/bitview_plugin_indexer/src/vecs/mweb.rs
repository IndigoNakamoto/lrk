use bitview_primitives::{Boolean, Count, Count32};
use bitview_traversable::Traversable;
use brk_error::Result;
use brk_types::{Height, Sats, Version};
use vecdb::{AnyStoredVec, Database, ImportableVec, PcoVec, Rw, StorageMode};

/// Per-block summaries of Litecoin's MWEB extension block, read from the block's serialized
/// extension (zero before activation and wherever it failed to decode).
#[derive(Traversable)]
pub struct MwebVecs<M: StorageMode = Rw> {
    /// Kernels in the extension block: one or more per MWEB transaction.
    #[traversable(lead = "MWEB kernels in the block.")]
    pub kernel_count: M::Stored<PcoVec<Height, Count32>>,
    /// Confidential outputs the extension block spends.
    #[traversable(lead = "MWEB inputs in the block.")]
    pub input_count: M::Stored<PcoVec<Height, Count32>>,
    /// Confidential outputs the extension block creates.
    #[traversable(lead = "MWEB outputs in the block.")]
    pub output_count: M::Stored<PcoVec<Height, Count32>>,
    /// Outputs ever created inside MWEB, from the extension header.
    #[traversable(lead = "MWEB outputs ever created.")]
    pub output_mmr_size: M::Stored<PcoVec<Height, Count>>,
    /// Kernels ever produced inside MWEB, from the extension header.
    #[traversable(lead = "MWEB kernels ever produced.")]
    pub kernel_mmr_size: M::Stored<PcoVec<Height, Count>>,
    /// Sum of the kernels' fees: what the HogEx leaves for the coinbase to claim.
    #[traversable(lead = "Fees paid inside MWEB.")]
    pub kernel_fee: M::Stored<PcoVec<Height, Sats>>,
    /// Sum of the kernels' peg-in amounts: coins entering MWEB, as MWEB records them.
    #[traversable(lead = "Peg-ins, as MWEB's kernels record them.")]
    pub kernel_pegin: M::Stored<PcoVec<Height, Sats>>,
    /// Sum of the kernels' peg-out amounts: coins leaving MWEB, as MWEB records them.
    #[traversable(lead = "Peg-outs, as MWEB's kernels record them.")]
    pub kernel_pegout: M::Stored<PcoVec<Height, Sats>>,
    /// Kernels carrying a peg-in.
    #[traversable(lead = "MWEB kernels pegging coins in.")]
    pub pegin_kernel_count: M::Stored<PcoVec<Height, Count32>>,
    /// Kernels carrying at least one peg-out.
    #[traversable(lead = "MWEB kernels pegging coins out.")]
    pub pegout_kernel_count: M::Stored<PcoVec<Height, Count32>>,
    /// The block signalled an extension block that failed to decode.
    #[traversable(hidden)]
    pub decode_failed: M::Stored<PcoVec<Height, Boolean>>,
    /// A block at or after activation without a HogEx as its last transaction.
    #[traversable(hidden)]
    pub missing_hogex: M::Stored<PcoVec<Height, Boolean>>,
}

impl MwebVecs {
    pub fn import(db: &Database, version: Version) -> Result<Self> {
        let (
            kernel_count,
            input_count,
            output_count,
            output_mmr_size,
            kernel_mmr_size,
            kernel_fee,
            kernel_pegin,
            kernel_pegout,
            pegin_kernel_count,
            pegout_kernel_count,
            decode_failed,
            missing_hogex,
        ) = parallel_import! {
            kernel_count = PcoVec::import(db, "mweb_kernel_count", version),
            input_count = PcoVec::import(db, "mweb_input_count", version),
            output_count = PcoVec::import(db, "mweb_output_count", version),
            output_mmr_size = PcoVec::import(db, "mweb_output_mmr_size", version),
            kernel_mmr_size = PcoVec::import(db, "mweb_kernel_mmr_size", version),
            kernel_fee = PcoVec::import(db, "mweb_kernel_fee", version),
            kernel_pegin = PcoVec::import(db, "mweb_kernel_pegin", version),
            kernel_pegout = PcoVec::import(db, "mweb_kernel_pegout", version),
            pegin_kernel_count = PcoVec::import(db, "mweb_pegin_kernel_count", version),
            pegout_kernel_count = PcoVec::import(db, "mweb_pegout_kernel_count", version),
            decode_failed = PcoVec::import(db, "mweb_decode_failed", version),
            missing_hogex = PcoVec::import(db, "mweb_missing_hogex", version),
        };
        Ok(Self {
            kernel_count,
            input_count,
            output_count,
            output_mmr_size,
            kernel_mmr_size,
            kernel_fee,
            kernel_pegin,
            kernel_pegout,
            pegin_kernel_count,
            pegout_kernel_count,
            decode_failed,
            missing_hogex,
        })
    }

    pub fn iter_any(&self) -> impl Iterator<Item = &dyn AnyStoredVec> {
        [
            &self.kernel_count as &dyn AnyStoredVec,
            &self.input_count,
            &self.output_count,
            &self.output_mmr_size,
            &self.kernel_mmr_size,
            &self.kernel_fee,
            &self.kernel_pegin,
            &self.kernel_pegout,
            &self.pegin_kernel_count,
            &self.pegout_kernel_count,
            &self.decode_failed,
            &self.missing_hogex,
        ]
        .into_iter()
    }
}
