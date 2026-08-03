use brk_error::Result;
use brk_types::{CapitalSentimentPhase, StoredI8, Version};
use vecdb::{Database, ReadableCloneableVec, UnaryTransform};

use super::Vecs;
use crate::{
    indexes,
    internal::{LazyPerBlock, PerBlock},
};

const VERSION: Version = Version::new(1);

struct CodeToPhase;

impl UnaryTransform<StoredI8, Option<CapitalSentimentPhase>> for CodeToPhase {
    #[inline]
    fn apply(code: StoredI8) -> Option<CapitalSentimentPhase> {
        if *code == 0 {
            None
        } else {
            Some(
                CapitalSentimentPhase::from_code(*code as u8)
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

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let version = parent_version + VERSION;

        let phase_code =
            PerBlock::forced_import(db, "capital_sentiment_phase_code", version, indexes)?;
        let phase = LazyPerBlock::from_computed::<CodeToPhase>(
            "capital_sentiment_phase",
            version,
            phase_code.height.read_only_boxed_clone(),
            &phase_code,
        );
        let score = LazyPerBlock::from_lazy::<PhaseToScore, StoredI8>(
            "capital_sentiment_score",
            version,
            &phase,
        );

        Ok(Self {
            phase_code,
            phase,
            score,
        })
    }
}
