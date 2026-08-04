use brk_error::Result;
use brk_types::{CapitalSentimentPhase, StoredBool, StoredI8, StoredU8, Version};
use vecdb::{Database, ReadableCloneableVec, UnaryTransform};

use super::Vecs;
use crate::{
    indexes,
    internal::{LazyPerBlock, PerBlock},
};

const VERSION: Version = Version::new(3);

struct CodeToPhase;

impl UnaryTransform<StoredU8, Option<CapitalSentimentPhase>> for CodeToPhase {
    #[inline]
    fn apply(code: StoredU8) -> Option<CapitalSentimentPhase> {
        if *code == 0 {
            None
        } else {
            Some(
                CapitalSentimentPhase::from_code(*code)
                    .expect("persisted Capital Sentiment phase code must be valid"),
            )
        }
    }
}

struct PhaseToScore;

impl UnaryTransform<Option<CapitalSentimentPhase>, Option<StoredI8>> for PhaseToScore {
    #[inline]
    fn apply(phase: Option<CapitalSentimentPhase>) -> Option<StoredI8> {
        phase.map(|phase| StoredI8::new(phase.score()))
    }
}

struct IsLongToIsShort;

impl UnaryTransform<StoredBool, StoredBool> for IsLongToIsShort {
    #[inline]
    fn apply(is_long: StoredBool) -> StoredBool {
        StoredBool::from(is_long.is_false())
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let version = parent_version + VERSION;

        let phase_code =
            PerBlock::forced_import(db, "capital_sentiment_phase_code", version, indexes)?;
        let is_long = PerBlock::<StoredBool>::forced_import(
            db,
            "capital_sentiment_is_long",
            version,
            indexes,
        )?;
        let is_short = LazyPerBlock::from_computed::<IsLongToIsShort>(
            "capital_sentiment_is_short",
            version,
            is_long.height.read_only_boxed_clone(),
            &is_long,
        );
        let phase = LazyPerBlock::from_computed::<CodeToPhase>(
            "capital_sentiment_phase",
            version,
            phase_code.height.read_only_boxed_clone(),
            &phase_code,
        );
        let score = LazyPerBlock::from_lazy::<PhaseToScore, StoredU8>(
            "capital_sentiment_score",
            version,
            &phase,
        );

        Ok(Self {
            phase_code,
            is_long,
            is_short,
            phase,
            score,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_is_the_lazy_complement_of_long() {
        assert!(IsLongToIsShort::apply(StoredBool::FALSE).is_true());
        assert!(IsLongToIsShort::apply(StoredBool::TRUE).is_false());
    }
}
