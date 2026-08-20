use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::Version;
use vecdb::{Database, LazyVecFrom1, ReadableCloneableVec};

use super::Vecs;
use crate::{
    indexes,
    internal::{
        BlocksToDaysF32, DifficultyToHashF64, ExplorerDifficultyF64, LazyPerBlock, PerBlock,
        PercentPerBlock, Resolutions,
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        // v3: Litecoin block-time / explorer-difficulty scale for hashrate & days.
        let v3 = Version::new(3);

        let hashrate = LazyPerBlock::from_height_source::<DifficultyToHashF64, _>(
            "difficulty_hashrate",
            version + v3,
            indexer.vecs.blocks.difficulty.read_only_clone(),
            indexes,
        );

        let blocks_to_retarget =
            PerBlock::forced_import(db, "blocks_to_retarget", version + v3, indexes)?;

        let days_to_retarget = LazyPerBlock::from_computed::<BlocksToDaysF32>(
            "days_to_retarget",
            version + v3,
            blocks_to_retarget.height.read_only_boxed_clone(),
            &blocks_to_retarget,
        );

        // Scale for resolution charts/API epochs only — do not register a Height
        // series named `difficulty` (indexer already owns that leaf).
        let explorer_difficulty = LazyVecFrom1::transformed::<ExplorerDifficultyF64>(
            "difficulty_explorer",
            version + v3,
            indexer.vecs.blocks.difficulty.read_only_boxed_clone(),
        );

        Ok(Self {
            value: Resolutions::forced_import(
                "difficulty",
                explorer_difficulty,
                version + v3,
                indexes,
            ),
            hashrate,
            adjustment: PercentPerBlock::forced_import(
                db,
                "difficulty_adjustment",
                version + Version::ONE,
                indexes,
            )?,
            epoch: PerBlock::forced_import(db, "difficulty_epoch", version, indexes)?,
            blocks_to_retarget,
            days_to_retarget,
        })
    }
}
