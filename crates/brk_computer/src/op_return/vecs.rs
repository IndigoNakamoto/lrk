use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{BasisPoints16, Height, StoredU64, VSize, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, AnyVec, Database, Exit, Rw, StorageMode, WritableVec};

use super::ByKind;
use crate::{
    indexes,
    internal::{PerBlockCumulativeRolling, PercentPerBlock, WindowStartVec, Windows},
};

pub type Series<T, M = Rw> = PerBlockCumulativeRolling<T, T, M>;

#[derive(Clone, Copy, Default)]
pub(super) struct Totals {
    pub output_count: u64,
    pub data_bytes: u64,
    pub tx_count: u64,
    pub tx_vsize: VSize,
}

#[derive(Traversable)]
pub struct TotalMetrics<M: StorageMode = Rw> {
    pub data_bytes: Series<StoredU64, M>,
    pub tx_count: Series<StoredU64, M>,
    pub tx_vsize: Series<VSize, M>,
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
        })
    }

    fn len(&self) -> usize {
        self.data_bytes
            .block
            .len()
            .min(self.tx_count.block.len())
            .min(self.tx_vsize.block.len())
    }

    pub(super) fn push(&mut self, block: Totals) {
        self.data_bytes.block.push(block.data_bytes.into());
        self.tx_count.block.push(block.tx_count.into());
        self.tx_vsize.block.push(block.tx_vsize);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.data_bytes
            .block
            .validate_and_truncate(version, height)?;
        self.tx_count.block.validate_and_truncate(version, height)?;
        self.tx_vsize.block.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.data_bytes.block.truncate_if_needed_at(len)?;
        self.tx_count.block.truncate_if_needed_at(len)?;
        self.tx_vsize.block.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.data_bytes.block.write()?;
        self.tx_count.block.write()?;
        self.tx_vsize.block.write()?;
        Ok(())
    }

    fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.data_bytes.compute_rest(max_from, exit)?;
        self.tx_count.compute_rest(max_from, exit)?;
        self.tx_vsize.compute_rest(max_from, exit)?;
        Ok(())
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Total<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub metrics: TotalMetrics<M>,
    pub chain_share: PercentPerBlock<BasisPoints16, M>,
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
        self.output_count.block.push(block.output_count.into());
        self.total.push(block);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.output_count
            .block
            .validate_and_truncate(version, height)?;
        self.total.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.output_count.block.truncate_if_needed_at(len)?;
        self.total.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.output_count.block.write()?;
        self.total.write()?;
        Ok(())
    }

    fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.output_count.compute_rest(max_from, exit)?;
        self.total.compute_cumulative(max_from, exit)?;
        Ok(())
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Breakdown<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub metrics: Metrics<M>,
    pub data_share: PercentPerBlock<BasisPoints16, M>,
    pub chain_share: PercentPerBlock<BasisPoints16, M>,
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

    pub(crate) fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.total.compute_cumulative(max_from, exit)?;
        for metrics in self.by_kind.iter_mut() {
            metrics.compute_cumulative(max_from, exit)?;
        }
        for metrics in self.policy.iter_mut() {
            metrics.compute_cumulative(max_from, exit)?;
        }
        Ok(())
    }
}
