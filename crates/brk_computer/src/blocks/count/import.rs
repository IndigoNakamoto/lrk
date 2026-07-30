use brk_indexer::Indexer;
use brk_types::{Height, StoredU64, Version, Weight};

use super::Vecs;
use crate::{
    indexes,
    internal::{
        BlockCountTarget1m, BlockCountTarget1w, BlockCountTarget1y, BlockCountTarget24h,
        ConstantVecs, LazyPerBlockCumulativeRolling, WindowStartVec, Windows,
    },
};

fn cumulative_block_count(height: Height, _: Weight) -> StoredU64 {
    StoredU64::from(u64::from(height) + 1)
}

impl Vecs {
    pub(crate) fn new(
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Self {
        Self {
            target: Windows {
                _24h: ConstantVecs::new::<BlockCountTarget24h>(
                    "block_count_target_24h",
                    version,
                    indexes,
                ),
                _1w: ConstantVecs::new::<BlockCountTarget1w>(
                    "block_count_target_1w",
                    version,
                    indexes,
                ),
                _1m: ConstantVecs::new::<BlockCountTarget1m>(
                    "block_count_target_1m",
                    version,
                    indexes,
                ),
                _1y: ConstantVecs::new::<BlockCountTarget1y>(
                    "block_count_target_1y",
                    version,
                    indexes,
                ),
            },
            total: LazyPerBlockCumulativeRolling::from_indexed_source(
                "block_count",
                version + Version::ONE,
                &indexer.vecs.blocks.weight,
                cumulative_block_count,
                cached_starts,
                indexes,
            ),
        }
    }
}
