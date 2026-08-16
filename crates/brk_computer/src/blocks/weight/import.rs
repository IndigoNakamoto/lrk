use brk_indexer::Indexer;
use brk_types::{Height, PartsPerMillion32, Version, Weight};

use super::Vecs;
use crate::{
    blocks::SizeVecs,
    indexes,
    internal::{
        CachedWindowStartVec, LazyPerBlockRolling, LazyPercentVec, VBytesToWeight, Windows,
    },
};

fn block_fullness(_: Height, weight: Weight) -> PartsPerMillion32 {
    PartsPerMillion32::from(weight.fullness())
}

impl Vecs {
    pub(crate) fn new(
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
        size: &SizeVecs,
    ) -> Self {
        let weight = LazyPerBlockRolling::from_full_parts::<VBytesToWeight>(
            "block_weight",
            version,
            &size.vbytes.cumulative,
            &size.vbytes.rolling,
            cached_starts,
            indexes,
        );

        let fullness = LazyPercentVec::from_indexed_source(
            "block_fullness",
            version,
            &indexer.vecs().blocks.weight,
            block_fullness,
        );

        Self { weight, fullness }
    }
}
