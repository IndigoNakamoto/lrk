use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::Height;
use schemars::JsonSchema;
use vecdb::{Database, EagerVec, Exit, ImportableVec, PcoVec, Rw, StorageMode, Version};

use crate::{
    indexes,
    internal::{NumericValue, PerBlock, RollingComplete, WindowStartVec, WindowStarts, Windows},
};

#[derive(Traversable)]
pub struct PerBlockAggregated<T, M: StorageMode = Rw>
where
    T: NumericValue + JsonSchema,
{
    pub sum: M::Stored<EagerVec<PcoVec<Height, T>>>,
    pub cumulative: PerBlock<T, M>,
    pub rolling: RollingComplete<T, M>,
}

impl<T> PerBlockAggregated<T>
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
        let sum = EagerVec::forced_import(db, &format!("{name}_sum"), version)?;
        let cumulative =
            PerBlock::forced_import(db, &format!("{name}_cumulative"), version, indexes)?;
        let rolling = RollingComplete::forced_import(
            db,
            name,
            version,
            indexes,
            &cumulative.height,
            cached_starts,
        )?;

        Ok(Self {
            sum,
            cumulative,
            rolling,
        })
    }

    pub(crate) fn compute_rest(
        &mut self,
        max_from: Height,
        windows: &WindowStarts<'_>,
        exit: &Exit,
    ) -> Result<()>
    where
        T: From<f64> + Default + Copy + Ord,
        f64: From<T>,
    {
        self.cumulative
            .height
            .compute_cumulative(max_from, &self.sum, exit)?;
        self.rolling.compute(max_from, windows, &self.sum, exit)?;
        Ok(())
    }
}
