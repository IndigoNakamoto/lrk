use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, ReadableCloneableVec};

use super::{DerivedVecs, Vecs};
use crate::{
    indexes,
    internal::{
        LazyPerBlock, OneMinusF64, PerBlock, PerBlockCumulativeRolling, WindowStartVec, Windows,
    },
};

impl DerivedVecs {
    pub(crate) fn forced_import_with_prefix(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let name = |metric: &str| {
            if prefix.is_empty() {
                metric.to_owned()
            } else {
                format!("{prefix}_{metric}")
            }
        };
        let liveliness_name = name("liveliness");
        let liveliness = PerBlock::forced_import(db, &liveliness_name, version, indexes)?;
        let vaultedness = LazyPerBlock::from_computed::<OneMinusF64>(
            &name("vaultedness"),
            version,
            liveliness.height.read_only_boxed_clone(),
            &liveliness,
        );

        Ok(Self {
            liveliness,
            vaultedness,
            ratio: PerBlock::forced_import(db, &name("activity_to_vaultedness"), version, indexes)?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            coinblocks_created: PerBlockCumulativeRolling::forced_import(
                db,
                "coinblocks_created",
                version,
                indexes,
                cached_starts,
            )?,
            coinblocks_stored: PerBlockCumulativeRolling::forced_import(
                db,
                "coinblocks_stored",
                version,
                indexes,
                cached_starts,
            )?,
            derived: DerivedVecs::forced_import_with_prefix(db, "", version, indexes)?,
        })
    }
}
