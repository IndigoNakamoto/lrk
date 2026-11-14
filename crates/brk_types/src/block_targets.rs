pub const TARGET_BLOCKS_PER_DAY_F64: f64 = TARGET_BLOCKS_PER_DAY as f64;
pub const TARGET_BLOCKS_PER_DAY_F32: f32 = TARGET_BLOCKS_PER_DAY as f32;
#[cfg(not(feature = "litecoin"))]
pub const TARGET_BLOCKS_PER_DAY: u64 = 144;
#[cfg(feature = "litecoin")]
pub const TARGET_BLOCKS_PER_DAY: u64 = 576;

/// Target seconds between blocks.
pub const TARGET_BLOCK_SECONDS: u64 = 86_400 / TARGET_BLOCKS_PER_DAY;

/// Target blocks per (365-day) year.
pub const TARGET_BLOCKS_PER_YEAR: u64 = 365 * TARGET_BLOCKS_PER_DAY;

/// A header's `difficulty_float()` in the convention explorers and Core's `getdifficulty` use
/// (difficulty 1 = target `0x1d00ffff`). The litecoin crate measures against Litecoin's powLimit
/// `0x1e0ffff0`, exactly 4096 times easier.
#[inline]
pub fn explorer_difficulty(header_difficulty_float: f64) -> f64 {
    if cfg!(feature = "litecoin") {
        header_difficulty_float / 4096.0
    } else {
        header_difficulty_float
    }
}
