use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, Version};
use vecdb::{
    AnyStoredVec, AnyVec, Database, Exit, ReadableVec, Rw, StorageMode, VecValue, WritableVec,
};

use crate::{
    indexes,
    internal::{
        FiatPerBlock, FiatType, LazyFiatBlock, LazyRollingSumsFiatFromHeight, WindowStartVec,
        Windows,
    },
};

#[derive(Traversable)]
pub struct FiatPerBlockCumulativeWithSums<C: FiatType, M: StorageMode = Rw> {
    pub block: LazyFiatBlock<C>,
    pub cumulative: FiatPerBlock<C, M>,
    pub sum: LazyRollingSumsFiatFromHeight<C>,
    #[traversable(skip)]
    last_cumulative: Option<(usize, C)>,
}

const VERSION: Version = Version::ONE;

impl<C: FiatType> FiatPerBlockCumulativeWithSums<C> {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let v = version + VERSION;
        let cumulative =
            FiatPerBlock::forced_import(db, &format!("{name}_cumulative"), v, indexes)?;
        let last_cumulative = cumulative
            .cents
            .height
            .collect_last()
            .map(|value| (cumulative.cents.height.len(), value));
        let block = LazyFiatBlock::from_cumulative(name, v, &cumulative);
        let sum = LazyRollingSumsFiatFromHeight::new(
            &format!("{name}_sum"),
            v,
            &cumulative.cents.height,
            cached_starts,
            indexes,
        );
        Ok(Self {
            block,
            cumulative,
            sum,
            last_cumulative,
        })
    }

    #[inline(always)]
    pub(crate) fn push_block(&mut self, value: C)
    where
        C: Copy,
    {
        let len = self.cumulative.cents.height.len();
        let mut cumulative = match self.last_cumulative {
            Some((cached_len, value)) if cached_len == len => value,
            _ => self
                .cumulative
                .cents
                .height
                .collect_last()
                .unwrap_or_default(),
        };
        cumulative += value;
        self.cumulative.cents.height.push(cumulative);
        self.last_cumulative = Some((len + 1, cumulative));
    }

    pub(crate) fn compute_from_cumulative_pair<S1, S2>(
        &mut self,
        max_from: Height,
        source1: &impl ReadableVec<Height, S1>,
        source2: &impl ReadableVec<Height, S2>,
        mut transform: impl FnMut(Height, S1, S2) -> C,
        exit: &Exit,
    ) -> Result<()>
    where
        S1: VecValue,
        S2: VecValue,
    {
        self.cumulative.cents.height.compute_transform2(
            max_from,
            source1,
            source2,
            |(height, value1, value2, ..)| (height, transform(height, value1, value2)),
            exit,
        )?;
        self.last_cumulative = None;
        Ok(())
    }

    pub(crate) fn compute_sum_of_others(
        &mut self,
        max_from: Height,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        self.cumulative.cents.height.compute_sum_of_others(
            max_from,
            &others
                .iter()
                .map(|v| &v.cumulative.cents.height)
                .collect::<Vec<_>>(),
            exit,
        )?;
        self.last_cumulative = None;
        Ok(())
    }

    pub(crate) fn stored_mut(&mut self) -> &mut dyn AnyStoredVec {
        self.last_cumulative = None;
        &mut self.cumulative.cents.height
    }
}
