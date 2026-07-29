use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, Sats, StoredU64, VSize, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyVec, Database, Rw, StorageMode};

use super::ByKind;
use crate::{
    indexes,
    internal::{
        PerBlockCumulativeRolling, PercentCumulativeRolling, PercentPerBlock, WindowStartVec,
        Windows,
    },
};

pub type Series<T, M = Rw> = PerBlockCumulativeRolling<T, M>;

#[derive(Clone, Copy, Default)]
pub(super) struct Totals {
    pub output_count: u64,
    pub data_bytes: u64,
    pub tx_count: u64,
    pub tx_vsize: VSize,
    pub fees: Sats,
}

#[derive(Traversable)]
pub struct TotalMetrics<M: StorageMode = Rw> {
    pub data_bytes: Series<StoredU64, M>,
    pub tx_count: Series<StoredU64, M>,
    pub tx_vsize: Series<VSize, M>,
    /// Full fees paid by carrier transactions. A transaction carrying multiple
    /// kinds contributes its fee to each kind, matching `tx_count` and `tx_vsize`.
    pub fees: Series<Sats, M>,
}

impl TotalMetrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            data_bytes: Series::forced_import(
                db,
                &format!("{prefix}_data_bytes"),
                version,
                indexes,
                cached_starts,
            )?,
            tx_count: Series::forced_import(
                db,
                &format!("{prefix}_tx_count"),
                version,
                indexes,
                cached_starts,
            )?,
            tx_vsize: Series::forced_import(
                db,
                &format!("{prefix}_tx_vsize"),
                version,
                indexes,
                cached_starts,
            )?,
            fees: Series::forced_import(
                db,
                &format!("{prefix}_fees"),
                version,
                indexes,
                cached_starts,
            )?,
        })
    }

    fn len(&self) -> usize {
        self.data_bytes
            .block
            .len()
            .min(self.tx_count.block.len())
            .min(self.tx_vsize.block.len())
            .min(self.fees.block.len())
    }

    pub(super) fn push(&mut self, block: Totals) {
        self.data_bytes.push_block(block.data_bytes.into());
        self.tx_count.push_block(block.tx_count.into());
        self.tx_vsize.push_block(block.tx_vsize);
        self.fees.push_block(block.fees);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.data_bytes.validate_and_truncate(version, height)?;
        self.tx_count.validate_and_truncate(version, height)?;
        self.tx_vsize.validate_and_truncate(version, height)?;
        self.fees.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.data_bytes.truncate_if_needed_at(len)?;
        self.tx_count.truncate_if_needed_at(len)?;
        self.tx_vsize.truncate_if_needed_at(len)?;
        self.fees.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.data_bytes.write()?;
        self.tx_count.write()?;
        self.tx_vsize.write()?;
        self.fees.write()?;
        Ok(())
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Total<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub metrics: TotalMetrics<M>,
    pub chain_share: PercentPerBlock<PartsPerMillion32, M>,
    /// Share of all transaction fees, based on cumulative and rolling fee sums.
    pub fee_share: PercentCumulativeRolling<PartsPerMillion32, M>,
}

impl Total {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            metrics: TotalMetrics::forced_import(db, prefix, version, indexes, cached_starts)?,
            chain_share: PercentPerBlock::forced_import(
                db,
                &format!("{prefix}_chain_share"),
                version,
                indexes,
            )?,
            fee_share: PercentCumulativeRolling::forced_import(
                db,
                &format!("{prefix}_fee_share"),
                version,
                indexes,
            )?,
        })
    }
}

#[derive(Traversable)]
pub struct Metrics<M: StorageMode = Rw> {
    pub output_count: Series<StoredU64, M>,
    #[traversable(flatten)]
    pub total: TotalMetrics<M>,
}

impl Metrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            output_count: Series::forced_import(
                db,
                &format!("{prefix}_output_count"),
                version,
                indexes,
                cached_starts,
            )?,
            total: TotalMetrics::forced_import(db, prefix, version, indexes, cached_starts)?,
        })
    }

    fn len(&self) -> usize {
        self.output_count.block.len().min(self.total.len())
    }

    pub(super) fn push(&mut self, block: Totals) {
        self.output_count.push_block(block.output_count.into());
        self.total.push(block);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.output_count.validate_and_truncate(version, height)?;
        self.total.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.output_count.truncate_if_needed_at(len)?;
        self.total.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.output_count.write()?;
        self.total.write()?;
        Ok(())
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Breakdown<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub metrics: Metrics<M>,
    pub data_share: PercentPerBlock<PartsPerMillion32, M>,
    pub chain_share: PercentPerBlock<PartsPerMillion32, M>,
    /// Share of all transaction fees, based on cumulative and rolling fee sums.
    pub fee_share: PercentCumulativeRolling<PartsPerMillion32, M>,
}

impl Breakdown {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            metrics: Metrics::forced_import(db, prefix, version, indexes, cached_starts)?,
            data_share: PercentPerBlock::forced_import(
                db,
                &format!("{prefix}_data_share"),
                version,
                indexes,
            )?,
            chain_share: PercentPerBlock::forced_import(
                db,
                &format!("{prefix}_chain_share"),
                version,
                indexes,
            )?,
            fee_share: PercentCumulativeRolling::forced_import(
                db,
                &format!("{prefix}_fee_share"),
                version,
                indexes,
            )?,
        })
    }
}

#[derive(Traversable)]
pub struct Policy<M: StorageMode = Rw> {
    pub pre_v30_standard: Breakdown<M>,
    pub pre_v30_nonstandard: Breakdown<M>,
    pub oversized: Breakdown<M>,
    pub multiple: Breakdown<M>,
}

impl Policy {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let import = |name| {
            Breakdown::forced_import(
                db,
                &format!("op_return_policy_{name}"),
                version,
                indexes,
                cached_starts,
            )
        };

        Ok(Self {
            pre_v30_standard: import("pre_v30_standard")?,
            pre_v30_nonstandard: import("pre_v30_nonstandard")?,
            oversized: import("oversized")?,
            multiple: import("multiple")?,
        })
    }

    fn iter(&self) -> impl Iterator<Item = &Breakdown> {
        [
            &self.pre_v30_standard,
            &self.pre_v30_nonstandard,
            &self.oversized,
            &self.multiple,
        ]
        .into_iter()
    }

    pub(super) fn iter_mut(&mut self) -> impl Iterator<Item = &mut Breakdown> {
        [
            &mut self.pre_v30_standard,
            &mut self.pre_v30_nonstandard,
            &mut self.oversized,
            &mut self.multiple,
        ]
        .into_iter()
    }
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    pub total: Total<M>,
    pub by_kind: ByKind<Breakdown<M>>,
    pub policy: Policy<M>,
}

impl Vecs {
    pub(crate) fn min_len(&self) -> usize {
        let len = self
            .by_kind
            .iter()
            .map(|metrics| metrics.len())
            .fold(self.total.len(), usize::min);
        self.policy
            .iter()
            .map(|metrics| metrics.len())
            .fold(len, usize::min)
    }

    pub(crate) fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.total.validate_and_truncate(version, height)?;
        for metrics in self.by_kind.iter_mut() {
            metrics.validate_and_truncate(version, height)?;
        }
        for metrics in self.policy.iter_mut() {
            metrics.validate_and_truncate(version, height)?;
        }
        Ok(())
    }

    pub(crate) fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.total.truncate_if_needed_at(len)?;
        for metrics in self.by_kind.iter_mut() {
            metrics.truncate_if_needed_at(len)?;
        }
        for metrics in self.policy.iter_mut() {
            metrics.truncate_if_needed_at(len)?;
        }
        Ok(())
    }

    pub(crate) fn write(&mut self) -> Result<()> {
        self.total.write()?;
        for metrics in self.by_kind.iter_mut() {
            metrics.write()?;
        }
        for metrics in self.policy.iter_mut() {
            metrics.write()?;
        }
        Ok(())
    }
}
