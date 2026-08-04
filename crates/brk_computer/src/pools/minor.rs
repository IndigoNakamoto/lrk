use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, PoolSlug, StoredU64};
use vecdb::{ReadableCloneableVec, Version};

use crate::{
    indexes,
    internal::{
        CachedWindowStartVec, Identity, LazyPerBlock, LazyPercentPerBlock, LazyPreviousDeltaVec,
        LazyRollingSumsFromHeight, Windows,
    },
};

use super::{PoolHeights, pool_heights::PoolCumulativeVec};

#[derive(Clone, Traversable)]
pub struct BlocksMined {
    pub block: LazyPreviousDeltaVec<Height, StoredU64>,
    pub cumulative: LazyPerBlock<StoredU64>,
    pub sum: LazyRollingSumsFromHeight<StoredU64>,
}

impl BlocksMined {
    fn forced_import(
        name: &str,
        slug: PoolSlug,
        pool_heights: PoolHeights,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Self {
        let cumulative_name = format!("{name}_cumulative");
        let cumulative_source = PoolCumulativeVec::new(&cumulative_name, slug, pool_heights);
        let cumulative = LazyPerBlock::from_uncached_height_source::<Identity<StoredU64>, _>(
            &cumulative_name,
            version,
            cumulative_source,
            indexes,
        );
        let block =
            LazyPreviousDeltaVec::new(name, version, cumulative.height.read_only_boxed_clone());
        let sum = LazyRollingSumsFromHeight::new_uncached(
            &format!("{name}_sum"),
            version,
            &cumulative.height,
            cached_starts,
            indexes,
        );

        Self {
            block,
            cumulative,
            sum,
        }
    }
}

fn pool_dominance(height: Height, blocks_mined: StoredU64) -> PartsPerMillion32 {
    PartsPerMillion32::from(u64::from(blocks_mined) as f64 / (u64::from(height) + 1) as f64)
}

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub blocks_mined: BlocksMined,
    pub dominance: LazyPercentPerBlock<PartsPerMillion32>,
}

impl Vecs {
    pub(crate) fn forced_import(
        slug: PoolSlug,
        pool_heights: PoolHeights,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Self {
        let suffix = |s: &str| format!("{}_{s}", slug);

        let blocks_mined = BlocksMined::forced_import(
            &suffix("blocks_mined"),
            slug,
            pool_heights,
            version + Version::ONE,
            indexes,
            cached_starts,
        );

        let dominance = LazyPercentPerBlock::from_uncached_indexed_source(
            &suffix("dominance"),
            version,
            &blocks_mined.cumulative.height,
            pool_dominance,
            indexes,
        );

        Self {
            blocks_mined,
            dominance,
        }
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
