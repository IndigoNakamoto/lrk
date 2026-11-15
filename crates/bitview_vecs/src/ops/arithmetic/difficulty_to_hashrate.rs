use bitview_primitives::{Difficulty, Hashrate};
use vecdb::UnaryTransform;

pub struct DifficultyToHashrate;

impl UnaryTransform<Difficulty, Hashrate> for DifficultyToHashrate {
    #[inline(always)]
    fn apply(difficulty: Difficulty) -> Hashrate {
        const MULTIPLIER: f64 = 4_294_967_296.0 / brk_types::TARGET_BLOCK_SECONDS as f64; // 2^32 / target
        Hashrate::from(*difficulty * MULTIPLIER)
    }
}
