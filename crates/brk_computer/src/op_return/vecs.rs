use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, StoredU64, VSize, Version};
use vecdb::{AnyStoredVec, AnyVec, Database, Exit, Rw, StorageMode, WritableVec};

use super::ByKind;
use crate::{
    indexes,
    internal::{PerBlockCumulativeRolling, WindowStartVec, Windows},
};

pub type Series<T, M = Rw> = PerBlockCumulativeRolling<T, T, M>;

#[derive(Clone, Copy, Default)]
pub(super) struct Totals {
    pub output_count: u64,
    pub post_op_return_bytes: u64,
    pub carrier_tx_count: u64,
    pub carrier_vsize: VSize,
}

#[derive(Traversable)]
pub struct TotalMetrics<M: StorageMode = Rw> {
    pub post_op_return_bytes: Series<StoredU64, M>,
    pub carrier_tx_count: Series<StoredU64, M>,
    pub carrier_vsize: Series<VSize, M>,
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
            post_op_return_bytes: Series::forced_import(
                db,
                &format!("{prefix}_post_op_return_bytes"),
                version,
                indexes,
                cached_starts,
            )?,
            carrier_tx_count: Series::forced_import(
                db,
                &format!("{prefix}_carrier_tx_count"),
                version,
                indexes,
                cached_starts,
            )?,
            carrier_vsize: Series::forced_import(
                db,
                &format!("{prefix}_carrier_vsize"),
                version,
                indexes,
                cached_starts,
            )?,
        })
    }

    fn len(&self) -> usize {
        self.post_op_return_bytes
            .block
            .len()
            .min(self.carrier_tx_count.block.len())
            .min(self.carrier_vsize.block.len())
    }

    pub(super) fn push(&mut self, block: Totals) {
        self.post_op_return_bytes
            .block
            .push(block.post_op_return_bytes.into());
        self.carrier_tx_count
            .block
            .push(block.carrier_tx_count.into());
        self.carrier_vsize.block.push(block.carrier_vsize);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.post_op_return_bytes
            .block
            .validate_and_truncate(version, height)?;
        self.carrier_tx_count
            .block
            .validate_and_truncate(version, height)?;
        self.carrier_vsize
            .block
            .validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.post_op_return_bytes.block.truncate_if_needed_at(len)?;
        self.carrier_tx_count.block.truncate_if_needed_at(len)?;
        self.carrier_vsize.block.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.post_op_return_bytes.block.write()?;
        self.carrier_tx_count.block.write()?;
        self.carrier_vsize.block.write()?;
        Ok(())
    }

    fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.post_op_return_bytes.compute_rest(max_from, exit)?;
        self.carrier_tx_count.compute_rest(max_from, exit)?;
        self.carrier_vsize.compute_rest(max_from, exit)?;
        Ok(())
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

#[derive(Traversable)]
pub struct Policy<M: StorageMode = Rw> {
    pub standard: Metrics<M>,
    pub oversized: Metrics<M>,
    pub multiple: Metrics<M>,
    pub pre_v30_nonstandard: Metrics<M>,
}

impl Policy {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let import = |name| {
            Metrics::forced_import(
                db,
                &format!("op_return_policy_{name}"),
                version,
                indexes,
                cached_starts,
            )
        };

        Ok(Self {
            standard: import("standard")?,
            oversized: import("oversized")?,
            multiple: import("multiple")?,
            pre_v30_nonstandard: import("pre_v30_nonstandard")?,
        })
    }

    fn iter(&self) -> impl Iterator<Item = &Metrics> {
        [
            &self.standard,
            &self.oversized,
            &self.multiple,
            &self.pre_v30_nonstandard,
        ]
        .into_iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Metrics> {
        [
            &mut self.standard,
            &mut self.oversized,
            &mut self.multiple,
            &mut self.pre_v30_nonstandard,
        ]
        .into_iter()
    }
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    pub total: TotalMetrics<M>,
    pub by_kind: ByKind<Metrics<M>>,
    pub policy: Policy<M>,
}

impl Vecs {
    pub(crate) fn min_len(&self) -> usize {
        self.by_kind
            .iter()
            .chain(self.policy.iter())
            .map(Metrics::len)
            .fold(self.total.len(), usize::min)
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
