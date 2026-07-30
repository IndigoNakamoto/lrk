use brk_error::Result;
use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, PoolSlug};
use derive_more::{Deref, DerefMut};
use vecdb::{BinaryTransform, Database, Exit, ReadableVec, Rw, StorageMode, Version};

use crate::{
    indexes,
    internal::{
        LazyPercentRollingWindows, MaskSats, ValuePerBlockCumulativeRolling, WindowStartVec,
        Windows,
    },
    mining, price,
};

use super::minor;

#[derive(Deref, DerefMut, Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub base: minor::Vecs<M>,

    pub rewards: ValuePerBlockCumulativeRolling<M>,
    #[traversable(rename = "dominance")]
    pub dominance_rolling: LazyPercentRollingWindows<PartsPerMillion32>,
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        slug: PoolSlug,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let suffix = |s: &str| format!("{}_{s}", slug);

        let base = minor::Vecs::forced_import(db, slug, version, indexes, cached_starts)?;

        let rewards = ValuePerBlockCumulativeRolling::forced_import(
            db,
            &suffix("rewards"),
            version,
            indexes,
            cached_starts,
        )?;

        let dominance_rolling = LazyPercentRollingWindows::from_cumulative_average(
            &suffix("dominance"),
            version,
            &base.blocks_mined.cumulative.height,
            cached_starts,
            indexes,
        );

        Ok(Self {
            base,
            rewards,
            dominance_rolling,
        })
    }

    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        pool: &impl ReadableVec<Height, PoolSlug>,
        prices: &price::Vecs,
        mining: &mining::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        self.base.compute(indexer, pool, exit)?;

        self.rewards.compute_from_pair(
            starting_height,
            prices,
            &self.base.blocks_mined.block,
            &mining.rewards.coinbase.block.sats,
            |_, mask, value| MaskSats::apply(mask, value),
            exit,
        )?;
        Ok(())
    }
}
