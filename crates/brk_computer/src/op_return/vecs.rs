use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, StoredU64, VSize, Version};
use schemars::JsonSchema;
use vecdb::{
    AnyStoredVec, AnyVec, Database, EagerVec, Exit, ImportableVec, PcoVec, Rw, StorageMode,
    WritableVec,
};

use super::ByKind;
use crate::{
    indexes,
    internal::{NumericValue, PerBlock},
};

#[derive(Traversable)]
pub struct Series<T, M: StorageMode = Rw>
where
    T: NumericValue + JsonSchema,
{
    pub block: M::Stored<EagerVec<PcoVec<Height, T>>>,
    pub cumulative: PerBlock<T, M>,
}

impl<T> Series<T>
where
    T: NumericValue + JsonSchema,
{
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let block = EagerVec::forced_import(db, name, version)?;
        let cumulative =
            PerBlock::forced_import(db, &format!("{name}_cumulative"), version, indexes)?;
        Ok(Self { block, cumulative })
    }

    fn len(&self) -> usize {
        self.block.len()
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.block.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.block.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn push(&mut self, block: T) {
        self.block.push(block);
    }

    fn write(&mut self) -> Result<()> {
        self.block.write()?;
        Ok(())
    }

    fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.cumulative
            .height
            .compute_cumulative(max_from, &self.block, exit)?;
        Ok(())
    }
}

#[derive(Clone, Copy, Default)]
pub(super) struct Totals {
    pub output_count: u64,
    pub post_op_return_bytes: u64,
    pub carrier_tx_count: u64,
    pub carrier_vsize: VSize,
}

#[derive(Traversable)]
pub struct Metrics<M: StorageMode = Rw> {
    pub output_count: Series<StoredU64, M>,
    pub post_op_return_bytes: Series<StoredU64, M>,
    pub carrier_tx_count: Series<StoredU64, M>,
    pub carrier_vsize: Series<VSize, M>,
}

impl Metrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            output_count: Series::forced_import(
                db,
                &format!("{prefix}_output_count"),
                version,
                indexes,
            )?,
            post_op_return_bytes: Series::forced_import(
                db,
                &format!("{prefix}_post_op_return_bytes"),
                version,
                indexes,
            )?,
            carrier_tx_count: Series::forced_import(
                db,
                &format!("{prefix}_carrier_tx_count"),
                version,
                indexes,
            )?,
            carrier_vsize: Series::forced_import(
                db,
                &format!("{prefix}_carrier_vsize"),
                version,
                indexes,
            )?,
        })
    }

    fn len(&self) -> usize {
        self.output_count
            .len()
            .min(self.post_op_return_bytes.len())
            .min(self.carrier_tx_count.len())
            .min(self.carrier_vsize.len())
    }

    pub(super) fn push(&mut self, block: Totals) {
        self.output_count.push(block.output_count.into());
        self.post_op_return_bytes
            .push(block.post_op_return_bytes.into());
        self.carrier_tx_count.push(block.carrier_tx_count.into());
        self.carrier_vsize.push(block.carrier_vsize);
    }

    fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        self.output_count.validate_and_truncate(version, height)?;
        self.post_op_return_bytes
            .validate_and_truncate(version, height)?;
        self.carrier_tx_count
            .validate_and_truncate(version, height)?;
        self.carrier_vsize.validate_and_truncate(version, height)?;
        Ok(())
    }

    fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.output_count.truncate_if_needed_at(len)?;
        self.post_op_return_bytes.truncate_if_needed_at(len)?;
        self.carrier_tx_count.truncate_if_needed_at(len)?;
        self.carrier_vsize.truncate_if_needed_at(len)?;
        Ok(())
    }

    fn write(&mut self) -> Result<()> {
        self.output_count.write()?;
        self.post_op_return_bytes.write()?;
        self.carrier_tx_count.write()?;
        self.carrier_vsize.write()?;
        Ok(())
    }

    fn compute_cumulative(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.output_count.compute_cumulative(max_from, exit)?;
        self.post_op_return_bytes
            .compute_cumulative(max_from, exit)?;
        self.carrier_tx_count.compute_cumulative(max_from, exit)?;
        self.carrier_vsize.compute_cumulative(max_from, exit)?;
        Ok(())
    }
}

#[derive(Traversable)]
pub struct Policy<M: StorageMode = Rw> {
    pub oversized: Metrics<M>,
    pub multiple: Metrics<M>,
    pub pre_v30_nonstandard: Metrics<M>,
}

impl Policy {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let import = |name| {
            Metrics::forced_import(db, &format!("op_return_policy_{name}"), version, indexes)
        };

        Ok(Self {
            oversized: import("oversized")?,
            multiple: import("multiple")?,
            pre_v30_nonstandard: import("pre_v30_nonstandard")?,
        })
    }

    fn iter(&self) -> impl Iterator<Item = &Metrics> {
        [&self.oversized, &self.multiple, &self.pre_v30_nonstandard].into_iter()
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Metrics> {
        [
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
    pub total: Metrics<M>,
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
