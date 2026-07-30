use brk_types::{Halving, Height, StoredU32, Version};
use vecdb::ReadOnlyClone;

use super::Vecs;
use crate::{
    indexes,
    internal::{BlocksToDaysF32, Identity, LazyPerBlock},
};

fn blocks_left_to_halving(height: Height, _: Halving) -> StoredU32 {
    StoredU32::from(height.left_before_next_halving())
}

impl Vecs {
    pub(crate) fn new(version: Version, indexes: &indexes::Vecs) -> Self {
        let v2 = Version::TWO;

        let epoch = LazyPerBlock::from_height_source::<Identity<Halving>, _>(
            "halving_epoch",
            version,
            indexes.height.halving.read_only_clone(),
            indexes,
        );
        let blocks_to_halving = LazyPerBlock::from_indexed_source(
            "blocks_to_halving",
            version + v2,
            &indexes.height.halving,
            blocks_left_to_halving,
            indexes,
        );

        let days_to_halving = LazyPerBlock::from_lazy::<BlocksToDaysF32, StoredU32>(
            "days_to_halving",
            version + v2,
            &blocks_to_halving,
        );

        Self {
            epoch,
            blocks_to_halving,
            days_to_halving,
        }
    }
}
