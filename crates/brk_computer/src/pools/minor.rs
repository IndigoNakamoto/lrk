use brk_error::Result;
use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, PoolSlug, StoredU64};
use vecdb::{Database, Exit, ReadableCloneableVec, ReadableVec, Rw, StorageMode, Version};

use crate::{
    indexes,
    internal::{
        LazyPercentPerBlock, LazyPreviousDeltaVec, LazyRollingSumsFromHeight, PerBlock,
        WindowStartVec, Windows,
    },
};

#[derive(Traversable)]
pub struct BlocksMined<M: StorageMode = Rw> {
    pub block: LazyPreviousDeltaVec<Height, StoredU64>,
    pub cumulative: PerBlock<StoredU64, M>,
    pub sum: LazyRollingSumsFromHeight<StoredU64>,
}

impl BlocksMined {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let cumulative =
            PerBlock::forced_import(db, &format!("{name}_cumulative"), version, indexes)?;
        let block =
            LazyPreviousDeltaVec::new(name, version, cumulative.height.read_only_boxed_clone());
        let sum = LazyRollingSumsFromHeight::new(
            &format!("{name}_sum"),
            version,
            &cumulative.height,
            cached_starts,
            indexes,
        );

        Ok(Self {
            block,
            cumulative,
            sum,
        })
    }
}

fn pool_dominance(height: Height, blocks_mined: StoredU64) -> PartsPerMillion32 {
    PartsPerMillion32::from(u64::from(blocks_mined) as f64 / (u64::from(height) + 1) as f64)
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    slug: PoolSlug,

    pub blocks_mined: BlocksMined<M>,
    pub dominance: LazyPercentPerBlock<PartsPerMillion32>,
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

        let blocks_mined = BlocksMined::forced_import(
            db,
            &suffix("blocks_mined"),
            version + Version::ONE,
            indexes,
            cached_starts,
        )?;

        let dominance = LazyPercentPerBlock::from_indexed_source(
            &suffix("dominance"),
            version,
            &blocks_mined.cumulative.height,
            pool_dominance,
            indexes,
        );

        Ok(Self {
            slug,
            blocks_mined,
            dominance,
        })
    }

    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        pool: &impl ReadableVec<Height, PoolSlug>,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        self.blocks_mined
            .cumulative
            .height
            .compute_cumulative_count(starting_height, pool, |id| *id == self.slug, exit)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominance_is_cumulative_share_of_chain() {
        assert_eq!(
            pool_dominance(Height::from(3_u32), StoredU64::from(1_u64)),
            PartsPerMillion32::from(0.25),
        );
        assert_eq!(
            pool_dominance(Height::from(9_u32), StoredU64::from(10_u64)),
            PartsPerMillion32::ONE,
        );
    }
}
