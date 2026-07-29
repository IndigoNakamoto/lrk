//! PerBlockCumulativeRolling - stored cumulative + lazy block and rolling views.
//!
//! The cumulative vector is the sole stored source of truth. Per-block values
//! and rolling sums/averages are all derived lazily from it.

use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, Version};
use schemars::JsonSchema;
use vecdb::{
    AnyStoredVec, AnyVec, Database, Exit, ReadableCloneableVec, ReadableVec, Rw, StorageMode,
    VecValue, WritableVec,
};

use crate::{
    indexes,
    internal::{
        LazyPreviousDeltaVec, LazyRollingAvgsFromHeight, LazyRollingSumsFromHeight, NumericValue,
        PerBlock, WindowStartVec, Windows,
    },
};

#[derive(Traversable)]
pub struct PerBlockCumulativeRolling<T, M: StorageMode = Rw>
where
    T: NumericValue + JsonSchema,
{
    pub block: LazyPreviousDeltaVec<Height, T>,
    pub cumulative: PerBlock<T, M>,
    pub sum: LazyRollingSumsFromHeight<T>,
    pub average: LazyRollingAvgsFromHeight<T>,
    #[traversable(skip)]
    last_cumulative: Option<(usize, T)>,
}

impl<T> PerBlockCumulativeRolling<T>
where
    T: NumericValue + JsonSchema,
{
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let cumulative =
            PerBlock::forced_import(db, &format!("{name}_cumulative"), version, indexes)?;
        let last_cumulative = cumulative
            .height
            .collect_last()
            .map(|value| (cumulative.height.len(), value));
        let cumulative_source = cumulative.height.read_only_boxed_clone();
        let block = LazyPreviousDeltaVec::new(name, version, cumulative_source);
        let sum = LazyRollingSumsFromHeight::new(
            &format!("{name}_sum"),
            version,
            &cumulative.height,
            cached_starts,
            indexes,
        );
        let average = LazyRollingAvgsFromHeight::new(
            &format!("{name}_average"),
            version,
            &cumulative.height,
            cached_starts,
            indexes,
        );

        Ok(Self {
            block,
            cumulative,
            sum,
            average,
            last_cumulative,
        })
    }

    #[inline(always)]
    pub(crate) fn push_block(&mut self, value: T)
    where
        T: Copy,
    {
        let len = self.cumulative.height.len();
        let mut cumulative = match self.last_cumulative {
            Some((cached_len, value)) if cached_len == len => value,
            _ => self.cumulative.height.collect_last().unwrap_or_default(),
        };
        cumulative += value;
        self.cumulative.height.push(cumulative);
        self.last_cumulative = Some((len + 1, cumulative));
    }

    pub(crate) fn compute_cumulative<S>(
        &mut self,
        max_from: Height,
        source: &impl ReadableVec<Height, S>,
        exit: &Exit,
    ) -> Result<()>
    where
        S: VecValue + Into<T>,
        T: Copy,
    {
        Ok(self
            .cumulative
            .height
            .compute_cumulative(max_from, source, exit)?)
    }

    pub(crate) fn compute_cumulative_transformed<S>(
        &mut self,
        max_from: Height,
        source: &impl ReadableVec<Height, S>,
        mut transform: impl FnMut(S) -> T,
        exit: &Exit,
    ) -> Result<()>
    where
        S: VecValue,
        T: Copy,
    {
        let mut cumulative = None;
        Ok(self.cumulative.height.compute_transform(
            max_from,
            source,
            |(height, value, this)| {
                let cumulative = cumulative.get_or_insert_with(|| {
                    height
                        .decremented()
                        .and_then(|height| this.collect_one(height))
                        .unwrap_or_default()
                });
                *cumulative += transform(value);
                (height, *cumulative)
            },
            exit,
        )?)
    }

    pub(crate) fn validate_computed_version_or_reset(&mut self, version: Version) -> Result<()> {
        self.cumulative
            .height
            .validate_computed_version_or_reset(version)?;
        Ok(())
    }

    pub(crate) fn validate_and_truncate(&mut self, version: Version, height: Height) -> Result<()> {
        Ok(self
            .cumulative
            .height
            .validate_and_truncate(version, height)?)
    }

    pub(crate) fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        Ok(self.cumulative.height.truncate_if_needed_at(len)?)
    }

    pub(crate) fn write(&mut self) -> Result<()> {
        self.cumulative.height.write()?;
        Ok(())
    }

    pub(crate) fn stored_mut(&mut self) -> &mut dyn AnyStoredVec {
        &mut self.cumulative.height
    }
}

#[cfg(test)]
mod tests {
    use brk_types::{Height, StoredU64, Version};
    use vecdb::{
        AnyStoredVec, Database, EagerVec, ImportableVec, PcoVec, ReadableCloneableVec, ReadableVec,
        WritableVec,
    };

    use crate::internal::LazyPreviousDeltaVec;

    #[test]
    fn lazy_block_is_the_delta_of_cumulative() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "brk-lazy-block-cumulative-{}-{suffix}",
            std::process::id()
        ));
        let db = Database::open(&path).unwrap();
        let mut cumulative: EagerVec<PcoVec<Height, StoredU64>> =
            EagerVec::forced_import(&db, "cumulative", Version::ONE).unwrap();

        for value in [1_u64, 3, 6] {
            cumulative.push(StoredU64::from(value));
        }
        cumulative.write().unwrap();

        let block = LazyPreviousDeltaVec::<Height, StoredU64>::new(
            "block",
            Version::ONE,
            cumulative.read_only_boxed_clone(),
        );

        assert_eq!(
            block.collect_range_at(0, 3),
            [1_u64, 2, 3].map(StoredU64::from)
        );

        drop(block);
        drop(cumulative);
        drop(db);
        std::fs::remove_dir_all(path).unwrap();
    }
}
