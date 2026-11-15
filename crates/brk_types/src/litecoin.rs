use std::sync::OnceLock;

use crate::Height;

/// First block with an MWEB extension block and a HogEx (`softforks.mweb.bip8.since`).
pub const MWEB_ACTIVATION_HEIGHT: Height = Height::new(2_265_984);

/// Header version bit signalling that an MWEB extension block may follow the transactions.
pub const MWEB_VERSION_BIT: i32 = 0x2000_0000;

/// A missing HogEx after activation, or an extension block that fails to decode, is a hard error:
/// neither happens over the full chain. `BITVIEW_MWEB_STRICT=0` degrades the MWEB series instead
/// of stalling the tip, for a node ahead of the decoder (a new extension block format).
pub fn mweb_strict() -> bool {
    static STRICT: OnceLock<bool> = OnceLock::new();
    *STRICT.get_or_init(|| {
        !std::env::var("BITVIEW_MWEB_STRICT").is_ok_and(|v| v == "0" || v.eq_ignore_ascii_case("false"))
    })
}
