use std::sync::OnceLock;

use crate::Height;

/// First block with an MWEB extension block and a HogEx (`softforks.mweb.bip8.since`).
pub const MWEB_ACTIVATION_HEIGHT: Height = Height::new(2_265_984);

/// Header version bit signalling that an MWEB extension block may follow the transactions.
pub const MWEB_VERSION_BIT: i32 = 0x2000_0000;

/// `BITVIEW_MWEB_STRICT=1` turns a missing HogEx after activation, or an extension block that
/// fails to decode, into a hard error. Off by default so a decoder gap degrades the MWEB series
/// instead of stalling the tip; flip it on once a full reindex has shown neither ever happens.
pub fn mweb_strict() -> bool {
    static STRICT: OnceLock<bool> = OnceLock::new();
    *STRICT.get_or_init(|| {
        std::env::var("BITVIEW_MWEB_STRICT").is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"))
    })
}
