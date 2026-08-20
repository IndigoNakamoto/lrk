/// rust-litecoin `difficulty_float()` uses powLimit `0x1e0ffff0`. Litecoin Core
/// `GetDifficulty` and explorers use Bitcoin's difficulty-1 target `0x1d00ffff`.
/// Ratio is exactly 4096.
#[cfg(feature = "litecoin")]
pub(crate) const DIFFICULTY_FLOAT_TO_EXPLORER: f64 = 4096.0;

#[inline]
pub(crate) fn explorer_difficulty(powlimit_based: f64) -> f64 {
    #[cfg(feature = "litecoin")]
    {
        powlimit_based / DIFFICULTY_FLOAT_TO_EXPLORER
    }
    #[cfg(not(feature = "litecoin"))]
    {
        powlimit_based
    }
}
