use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, Sats, StoredU64, VSize, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyVec, CachedBoxedVec, Database, ReadOnlyClone, Rw, StorageMode};

use super::ByKind;
use crate::{
    indexes,
    internal::{
        CachedPerBlockCumulativeRolling, CachedWindowStartVec, LazyPercentCumulativeRolling,
        LazyPercentPerBlock, PerBlockCumulativeRolling, RatioSats, RatioU64, Windows,
    },
};

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
    pub data_bytes: PerBlockCumulativeRolling<StoredU64, M>,
    pub tx_count: PerBlockCumulativeRolling<StoredU64, M>,
    pub tx_vsize: PerBlockCumulativeRolling<VSize, M>,
    /// Full fees paid by carrier transactions. A transaction carrying multiple
    /// kinds contributes its fee to each kind, matching `tx_count` and `tx_vsize`.
    pub fees: PerBlockCumulativeRolling<Sats, M>,
}

impl TotalMetrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            data_bytes: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{prefix}_data_bytes"),
                version,
                indexes,
                cached_starts,
            )?,
            tx_count: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{prefix}_tx_count"),
                version,
                indexes,
                cached_starts,
            )?,
            tx_vsize: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{prefix}_tx_vsize"),
                version,
                indexes,
                cached_starts,
            )?,
            fees: PerBlockCumulativeRolling::forced_import(
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

    fn lazy_fee_share(
        &self,
        prefix: &str,
        version: Version,
        chain_fees: CachedBoxedVec<Height, Sats>,
        cached_starts: &Windows<&CachedWindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> LazyPercentCumulativeRolling<PartsPerMillion32> {
        LazyPercentCumulativeRolling::from_cumulative_ratio::<
            Sats,
            Sats,
            RatioSats<PartsPerMillion32>,
        >(
            &format!("{prefix}_fee_share"),
            version,
            &self.fees.cumulative.height,
            chain_fees,
            cached_starts,
            indexes,
        )
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

#[derive(Traversable)]
pub struct Total<M: StorageMode = Rw> {
    pub data_bytes: CachedPerBlockCumulativeRolling<StoredU64, M>,
    pub tx_count: PerBlockCumulativeRolling<StoredU64, M>,
    pub tx_vsize: PerBlockCumulativeRolling<VSize, M>,
    /// Full fees paid by carrier transactions. A transaction carrying multiple
    /// kinds contributes its fee to each kind, matching `tx_count` and `tx_vsize`.
    pub fees: PerBlockCumulativeRolling<Sats, M>,
    pub chain_share: LazyPercentPerBlock<PartsPerMillion32>,
    /// Share of all transaction fees, based on cumulative and rolling fee sums.
    pub fee_share: LazyPercentCumulativeRolling<PartsPerMillion32>,
}

impl Total {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
        block_size: &CachedBoxedVec<Height, StoredU64>,
        chain_fees: &CachedBoxedVec<Height, Sats>,
    ) -> Result<Self> {
        let data_bytes = CachedPerBlockCumulativeRolling::forced_import(
            db,
            &format!("{prefix}_data_bytes"),
            version,
            indexes,
            cached_starts,
        )?;
        let tx_count = PerBlockCumulativeRolling::forced_import(
            db,
            &format!("{prefix}_tx_count"),
            version,
            indexes,
            cached_starts,
        )?;
        let tx_vsize = PerBlockCumulativeRolling::forced_import(
            db,
            &format!("{prefix}_tx_vsize"),
            version,
            indexes,
            cached_starts,
        )?;
        let fees = PerBlockCumulativeRolling::forced_import(
            db,
            &format!("{prefix}_fees"),
            version,
            indexes,
            cached_starts,
        )?;

        Ok(Self {
            chain_share: Self::lazy_chain_share(
                prefix,
                version,
                &data_bytes,
                block_size.clone(),
                indexes,
            ),
            fee_share: Self::lazy_fee_share(
                prefix,
                version,
                &fees,
                chain_fees.clone(),
                cached_starts,
                indexes,
            ),
            data_bytes,
            tx_count,
            tx_vsize,
            fees,
        })
    }

    fn lazy_chain_share(
        prefix: &str,
        version: Version,
        data_bytes: &CachedPerBlockCumulativeRolling<StoredU64>,
        block_size: CachedBoxedVec<Height, StoredU64>,
        indexes: &indexes::Vecs,
    ) -> LazyPercentPerBlock<PartsPerMillion32> {
        let data_bytes = data_bytes.cumulative.height.read_only_clone();
        LazyPercentPerBlock::from_cached_ratio::<StoredU64, StoredU64, RatioU64<PartsPerMillion32>>(
            &format!("{prefix}_chain_share"),
            version,
            &data_bytes,
            block_size,
            indexes,
        )
    }

    fn lazy_fee_share(
        prefix: &str,
        version: Version,
        fees: &PerBlockCumulativeRolling<Sats>,
        chain_fees: CachedBoxedVec<Height, Sats>,
        cached_starts: &Windows<&CachedWindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> LazyPercentCumulativeRolling<PartsPerMillion32> {
        LazyPercentCumulativeRolling::from_cumulative_ratio::<
            Sats,
            Sats,
            RatioSats<PartsPerMillion32>,
        >(
            &format!("{prefix}_fee_share"),
            version,
            &fees.cumulative.height,
            chain_fees,
            cached_starts,
            indexes,
        )
    }

    pub(crate) fn cached_data_bytes(&self) -> CachedBoxedVec<Height, StoredU64> {
        self.data_bytes.cached_cumulative()
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

#[derive(Traversable)]
pub struct Metrics<M: StorageMode = Rw> {
    pub output_count: PerBlockCumulativeRolling<StoredU64, M>,
    #[traversable(flatten)]
    pub total: TotalMetrics<M>,
}

impl Metrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            output_count: PerBlockCumulativeRolling::forced_import(
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

    fn lazy_data_share(
        &self,
        name: &str,
        version: Version,
        denominator: CachedBoxedVec<Height, StoredU64>,
        indexes: &indexes::Vecs,
    ) -> LazyPercentPerBlock<PartsPerMillion32> {
        LazyPercentPerBlock::from_cached_ratio::<StoredU64, StoredU64, RatioU64<PartsPerMillion32>>(
            name,
            version,
            &self.total.data_bytes.cumulative.height,
            denominator,
            indexes,
        )
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
    pub data_share: LazyPercentPerBlock<PartsPerMillion32>,
    pub chain_share: LazyPercentPerBlock<PartsPerMillion32>,
    /// Share of all transaction fees, based on cumulative and rolling fee sums.
    pub fee_share: LazyPercentCumulativeRolling<PartsPerMillion32>,
}

pub(super) struct BreakdownImporter<'a> {
    db: &'a Database,
    version: Version,
    indexes: &'a indexes::Vecs,
    cached_starts: &'a Windows<&'a CachedWindowStartVec>,
    total_data: &'a CachedBoxedVec<Height, StoredU64>,
    block_size: &'a CachedBoxedVec<Height, StoredU64>,
    chain_fees: &'a CachedBoxedVec<Height, Sats>,
}

impl<'a> BreakdownImporter<'a> {
    pub(super) fn new(
        db: &'a Database,
        version: Version,
        indexes: &'a indexes::Vecs,
        cached_starts: &'a Windows<&'a CachedWindowStartVec>,
        total_data: &'a CachedBoxedVec<Height, StoredU64>,
        block_size: &'a CachedBoxedVec<Height, StoredU64>,
        chain_fees: &'a CachedBoxedVec<Height, Sats>,
    ) -> Self {
        Self {
            db,
            version,
            indexes,
            cached_starts,
            total_data,
            block_size,
            chain_fees,
        }
    }

    pub(super) fn import(&self, prefix: &str) -> Result<Breakdown> {
        let metrics = Metrics::forced_import(
            self.db,
            prefix,
            self.version,
            self.indexes,
            self.cached_starts,
        )?;
        let fee_share = metrics.total.lazy_fee_share(
            prefix,
            self.version,
            self.chain_fees.clone(),
            self.cached_starts,
            self.indexes,
        );

        Ok(Breakdown {
            data_share: metrics.lazy_data_share(
                &format!("{prefix}_data_share"),
                self.version,
                self.total_data.clone(),
                self.indexes,
            ),
            chain_share: metrics.lazy_data_share(
                &format!("{prefix}_chain_share"),
                self.version,
                self.block_size.clone(),
                self.indexes,
            ),
            metrics,
            fee_share,
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
    pub(super) fn forced_import(importer: &BreakdownImporter<'_>) -> Result<Self> {
        let import = |name| importer.import(&format!("op_return_policy_{name}"));

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
