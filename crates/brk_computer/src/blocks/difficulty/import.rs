use brk_indexer::Indexer;
use brk_types::{Epoch, Height, PartsPerMillionSigned32, StoredF64, StoredU32, Version};
use vecdb::{LazyVec, ReadOnlyClone, ReadableCloneableVec};

use super::Vecs;
use crate::{
    indexes,
    internal::{
        BlocksToDaysF32, DifficultyToHashF64, ExplorerDifficultyF64, Identity, LazyPerBlock,
        LazyPercentPerBlock, Resolutions,
    },
};

const DIFFICULTY_ADJUSTMENT_LOOKBACK: usize = 2016;

fn blocks_left_to_retarget(height: Height, _: Epoch) -> StoredU32 {
    StoredU32::from(height.left_before_next_diff_adj())
}

fn difficulty_adjustment(
    current: StoredF64,
    previous: Option<StoredF64>,
) -> PartsPerMillionSigned32 {
    match previous {
        Some(previous) => {
            PartsPerMillionSigned32::from((f32::from(current) / f32::from(previous)) - 1.0)
        }
        None => PartsPerMillionSigned32::from(f32::NAN),
    }
}

impl Vecs {
    pub(crate) fn new(version: Version, indexer: &Indexer, indexes: &indexes::Vecs) -> Self {
        // v3: Litecoin block-time / explorer-difficulty scale for hashrate & days.
        let v3 = Version::new(3);

        let hashrate = LazyPerBlock::from_height_source::<DifficultyToHashF64, _>(
            "difficulty_hashrate",
            version + v3,
            indexer.vecs().blocks.difficulty.read_only_clone(),
            indexes,
        );

        let epoch = LazyPerBlock::from_height_source::<Identity<Epoch>, _>(
            "difficulty_epoch",
            version,
            indexes.height.epoch.read_only_clone(),
            indexes,
        );
        let blocks_to_retarget = LazyPerBlock::from_indexed_source(
            "blocks_to_retarget",
            version + Version::TWO,
            &indexes.height.epoch,
            blocks_left_to_retarget,
            indexes,
        );

        let days_to_retarget = LazyPerBlock::from_lazy::<BlocksToDaysF32, StoredU32>(
            "days_to_retarget",
            version + v3,
            &blocks_to_retarget,
        );

        // Scale for resolution charts/API epochs only — do not register a Height
        // series named `difficulty` (indexer already owns that leaf).
        let explorer_difficulty = LazyVec::transformed::<ExplorerDifficultyF64>(
            "difficulty_explorer",
            version + v3,
            indexer.vecs().blocks.difficulty.read_only_boxed_clone(),
        );

        Self {
            value: Resolutions::forced_import(
                "difficulty",
                explorer_difficulty,
                version + v3,
                indexes,
            ),
            hashrate,
            adjustment: LazyPercentPerBlock::from_lookback_source(
                "difficulty_adjustment",
                version + Version::ONE,
                &indexer.vecs().blocks.difficulty,
                DIFFICULTY_ADJUSTMENT_LOOKBACK,
                difficulty_adjustment,
                indexes,
            ),
            epoch,
            blocks_to_retarget,
            days_to_retarget,
        }
    }
}
