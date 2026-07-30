//! Address activity tracking - per-block counts of address behaviors.
//!
//! Tracks global and per-address-type activity metrics:
//!
//! | Metric | Description |
//! |--------|-------------|
//! | `receiving` | Unique addresses that received this block |
//! | `sending` | Unique addresses that sent this block |
//! | `reactivated` | Addresses that were empty and now have funds |
//! | `bidirectional` | Addresses that both sent AND received in same block |
//! | `active` | Distinct addresses involved (sent ∪ received) |

use brk_cohort::ByAddrType;
use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::Version;
use derive_more::{Deref, DerefMut};
use rayon::prelude::*;
use vecdb::{AnyStoredVec, AnyVec, Database, Rw, StorageMode};

use crate::{
    indexes,
    internal::{CountPerBlockRollingAverage, WindowStartVec, Windows},
};

/// Per-block activity counts - reset each block.
#[derive(Debug, Default, Clone)]
pub struct BlockActivityCounts {
    pub reactivated: u32,
    pub sending: u32,
    pub receiving: u32,
    pub bidirectional: u32,
}

impl BlockActivityCounts {
    /// Reset all counts to zero.
    #[inline]
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Per-address-type activity counts - aggregated during block processing.
#[derive(Debug, Default, Deref, DerefMut)]
pub struct AddrTypeToActivityCounts(pub ByAddrType<BlockActivityCounts>);

impl AddrTypeToActivityCounts {
    /// Reset all per-type counts.
    pub(crate) fn reset(&mut self) {
        self.0.values_mut().for_each(|v| v.reset());
    }

    /// Sum all types to get totals.
    pub(crate) fn totals(&self) -> BlockActivityCounts {
        let mut total = BlockActivityCounts::default();
        for counts in self.0.values() {
            total.reactivated += counts.reactivated;
            total.sending += counts.sending;
            total.receiving += counts.receiving;
            total.bidirectional += counts.bidirectional;
        }
        total
    }
}

/// Activity count vectors for a single category (e.g., one address type or "all").
#[derive(Traversable)]
pub struct ActivityCountVecs<M: StorageMode = Rw> {
    pub reactivated: CountPerBlockRollingAverage<M>,
    pub sending: CountPerBlockRollingAverage<M>,
    pub receiving: CountPerBlockRollingAverage<M>,
    pub bidirectional: CountPerBlockRollingAverage<M>,
    /// Distinct addresses involved in this block (sent ∪ received),
    /// computed at push time as `sending + receiving - bidirectional`
    /// via inclusion-exclusion. For per-type instances this is
    /// per-type. For the `all` aggregate it's the cross-type total.
    pub active: CountPerBlockRollingAverage<M>,
}

impl ActivityCountVecs {
    /// `prefix` is prepended to each field's disk name. Use `""` for the
    /// "all" aggregate and `"{type}_"` for per-address-type instances.
    /// Field names are suffixed with `_addrs` so the final disk series
    /// are e.g. `active_addrs`, `p2tr_bidirectional_addrs`.
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            reactivated: CountPerBlockRollingAverage::forced_import(
                db,
                &format!("{prefix}reactivated_addrs"),
                version,
                indexes,
                cached_starts,
            )?,
            sending: CountPerBlockRollingAverage::forced_import(
                db,
                &format!("{prefix}sending_addrs"),
                version,
                indexes,
                cached_starts,
            )?,
            receiving: CountPerBlockRollingAverage::forced_import(
                db,
                &format!("{prefix}receiving_addrs"),
                version,
                indexes,
                cached_starts,
            )?,
            bidirectional: CountPerBlockRollingAverage::forced_import(
                db,
                &format!("{prefix}bidirectional_addrs"),
                version,
                indexes,
                cached_starts,
            )?,
            active: CountPerBlockRollingAverage::forced_import(
                db,
                &format!("{prefix}active_addrs"),
                version,
                indexes,
                cached_starts,
            )?,
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.reactivated
            .cumulative
            .len()
            .min(self.sending.cumulative.len())
            .min(self.receiving.cumulative.len())
            .min(self.bidirectional.cumulative.len())
            .min(self.active.cumulative.len())
    }

    pub(crate) fn par_iter_height_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = &mut dyn AnyStoredVec> {
        [
            self.reactivated.stored_mut(),
            self.sending.stored_mut(),
            self.receiving.stored_mut(),
            self.bidirectional.stored_mut(),
            self.active.stored_mut(),
        ]
        .into_par_iter()
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        self.reactivated.reset()?;
        self.sending.reset()?;
        self.receiving.reset()?;
        self.bidirectional.reset()?;
        self.active.reset()?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn push_height(&mut self, counts: &BlockActivityCounts) {
        self.reactivated.push_block(counts.reactivated.into());
        self.sending.push_block(counts.sending.into());
        self.receiving.push_block(counts.receiving.into());
        self.bidirectional.push_block(counts.bidirectional.into());
        let active = counts.sending + counts.receiving - counts.bidirectional;
        self.active.push_block(active.into());
    }
}

/// Per-address-type activity count vecs.
#[derive(Deref, DerefMut, Traversable)]
pub struct AddrTypeToActivityCountVecs<M: StorageMode = Rw>(ByAddrType<ActivityCountVecs<M>>);

impl From<ByAddrType<ActivityCountVecs>> for AddrTypeToActivityCountVecs {
    #[inline]
    fn from(value: ByAddrType<ActivityCountVecs>) -> Self {
        Self(value)
    }
}

impl AddrTypeToActivityCountVecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self::from(ByAddrType::<ActivityCountVecs>::new_with_name(
            |type_name| {
                ActivityCountVecs::forced_import(
                    db,
                    &format!("{type_name}_"),
                    version,
                    indexes,
                    cached_starts,
                )
            },
        )?))
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.0
            .values()
            .map(|v| v.min_stateful_len())
            .min()
            .unwrap_or(0)
    }

    pub(crate) fn par_iter_height_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = &mut dyn AnyStoredVec> {
        let mut vecs: Vec<&mut dyn AnyStoredVec> = Vec::new();
        for type_vecs in self.0.values_mut() {
            vecs.push(type_vecs.reactivated.stored_mut());
            vecs.push(type_vecs.sending.stored_mut());
            vecs.push(type_vecs.receiving.stored_mut());
            vecs.push(type_vecs.bidirectional.stored_mut());
            vecs.push(type_vecs.active.stored_mut());
        }
        vecs.into_par_iter()
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        for v in self.0.values_mut() {
            v.reset_height()?;
        }
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn push_height(&mut self, counts: &AddrTypeToActivityCounts) {
        for (vecs, c) in self.0.values_mut().zip(counts.0.values()) {
            vecs.push_height(c);
        }
    }
}

/// Storage for activity metrics (global + per type).
#[derive(Traversable)]
pub struct AddrActivityVecs<M: StorageMode = Rw> {
    pub all: ActivityCountVecs<M>,
    #[traversable(flatten)]
    pub by_addr_type: AddrTypeToActivityCountVecs<M>,
}

impl AddrActivityVecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            all: ActivityCountVecs::forced_import(db, "", version, indexes, cached_starts)?,
            by_addr_type: AddrTypeToActivityCountVecs::forced_import(
                db,
                version,
                indexes,
                cached_starts,
            )?,
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.all
            .min_stateful_len()
            .min(self.by_addr_type.min_stateful_len())
    }

    pub(crate) fn par_iter_height_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = &mut dyn AnyStoredVec> {
        self.all
            .par_iter_height_mut()
            .chain(self.by_addr_type.par_iter_height_mut())
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        self.all.reset_height()?;
        self.by_addr_type.reset_height()?;
        Ok(())
    }

    #[inline(always)]
    pub(crate) fn push_height(&mut self, counts: &AddrTypeToActivityCounts) {
        let totals = counts.totals();
        self.all.push_height(&totals);
        self.by_addr_type.push_height(counts);
    }
}
