//! Per-block values with rolling averages derived from cumulative prefix sums.
//!
//! Exact integer/timestamp metrics use a cumulative source of truth and a lazy
//! block view. Float metrics retain their stored block values because
//! subtracting cumulative floats would not reproduce them exactly.

use brk_error::Result;

use brk_traversable::Traversable;
use brk_types::{Height, StoredU32, StoredU64, Version};
use schemars::JsonSchema;
use vecdb::{
    AnyStoredVec, AnyVec, Database, EagerVec, Exit, Ident, ImportableVec, PcoVec,
    ReadableCloneableVec, ReadableVec, Rw, StorageMode, UnaryTransform, VecValue, WritableVec,
};

use crate::indexes;

use crate::internal::{
    CachedWindowStartVec, LazyPreviousDeltaVec, LazyRollingAvgsFromHeight, NumericValue,
    StoredU64ToStoredU32, Windows,
};

/// Cumulative source of truth with lazy exact per-block values and rolling averages.
#[derive(Traversable)]
pub struct PerBlockCumulativeAverage<T, C = T, M: StorageMode = Rw, F = Ident>
where
    T: NumericValue + JsonSchema,
    C: NumericValue + JsonSchema,
    F: UnaryTransform<C, T>,
{
    pub block: LazyPreviousDeltaVec<Height, C, T, F>,
    #[traversable(hidden)]
    cumulative: M::Stored<EagerVec<PcoVec<Height, C>>>,
    #[traversable(flatten)]
    pub average: LazyRollingAvgsFromHeight<C>,
    #[traversable(skip)]
    last_cumulative: Option<(usize, C)>,
}

pub type CountPerBlockRollingAverage<M = Rw> =
    PerBlockCumulativeAverage<StoredU32, StoredU64, M, StoredU64ToStoredU32>;

impl<T, C, F> PerBlockCumulativeAverage<T, C, Rw, F>
where
    T: NumericValue + JsonSchema + Into<C>,
    C: NumericValue + JsonSchema,
    F: UnaryTransform<C, T>,
{
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let cumulative_version = version + Version::TWO;
        let cumulative: EagerVec<PcoVec<Height, C>> =
            EagerVec::forced_import(db, &format!("{name}_cumulative"), cumulative_version)?;
        let last_cumulative = cumulative
            .collect_last()
            .map(|value| (cumulative.len(), value));
        let block =
            LazyPreviousDeltaVec::transformed(name, version, cumulative.read_only_boxed_clone());
        let average = LazyRollingAvgsFromHeight::new(
            &format!("{name}_average"),
            cumulative_version,
            &cumulative,
            cached_starts,
            indexes,
        );

        Ok(Self {
            block,
            cumulative,
            average,
            last_cumulative,
        })
    }

    #[inline(always)]
    pub(crate) fn push_block(&mut self, value: T)
    where
        C: Copy,
    {
        let len = self.cumulative.len();
        let mut cumulative = match self.last_cumulative {
            Some((cached_len, value)) if cached_len == len => value,
            _ => self.cumulative.collect_last().unwrap_or_default(),
        };
        cumulative += value.into();
        self.cumulative.push(cumulative);
        self.last_cumulative = Some((len + 1, cumulative));
    }

    pub(crate) fn compute_from<S>(
        &mut self,
        max_from: Height,
        source: &impl ReadableVec<Height, S>,
        mut transform: impl FnMut(Height, S) -> T,
        exit: &Exit,
    ) -> Result<()>
    where
        S: VecValue,
        C: Copy,
    {
        let mut cumulative = None;
        self.cumulative.compute_transform(
            max_from,
            source,
            |(height, value, this)| {
                let cumulative = cumulative.get_or_insert_with(|| {
                    height
                        .decremented()
                        .and_then(|height| this.collect_one(height))
                        .unwrap_or_default()
                });
                *cumulative += transform(height, value).into();
                (height, *cumulative)
            },
            exit,
        )?;
        self.last_cumulative = None;
        Ok(())
    }

    pub(crate) fn reset(&mut self) -> Result<()> {
        self.last_cumulative = None;
        self.cumulative.reset()?;
        Ok(())
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.cumulative.len()
    }

    pub(crate) fn stored_mut(&mut self) -> &mut dyn AnyStoredVec {
        self.last_cumulative = None;
        &mut self.cumulative
    }
}

/// Stored-block fallback for values whose cumulative delta is not exact, such as floats.
#[derive(Traversable)]
pub struct PerBlockRollingAverage<T, C = T, M: StorageMode = Rw>
where
    T: NumericValue + JsonSchema,
    C: NumericValue + JsonSchema,
{
    pub block: M::Stored<EagerVec<PcoVec<Height, T>>>,
    #[traversable(hidden)]
    cumulative: M::Stored<EagerVec<PcoVec<Height, C>>>,
    #[traversable(flatten)]
    pub average: LazyRollingAvgsFromHeight<C>,
}

impl<T, C> PerBlockRollingAverage<T, C>
where
    T: NumericValue + JsonSchema + Into<C>,
    C: NumericValue + JsonSchema,
{
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let block: EagerVec<PcoVec<Height, T>> = EagerVec::forced_import(db, name, version)?;
        let cumulative: EagerVec<PcoVec<Height, C>> =
            EagerVec::forced_import(db, &format!("{name}_cumulative"), version + Version::TWO)?;
        let average = LazyRollingAvgsFromHeight::new(
            &format!("{name}_average"),
            version + Version::TWO,
            &cumulative,
            cached_starts,
            indexes,
        );

        Ok(Self {
            block,
            cumulative,
            average,
        })
    }

    /// Compute cumulative from already-populated height data. Rolling averages are lazy.
    pub(crate) fn compute_rest(&mut self, max_from: Height, exit: &Exit) -> Result<()> {
        self.cumulative
            .compute_cumulative(max_from, &self.block, exit)?;
        Ok(())
    }
}
