//! The workspace depends on this crate under the name `bitcoin`
//! (`bitcoin = { package = "brk_chain", ... }`), so every `bitcoin::` path in
//! Bitview resolves to the `bitcoin` crate by default and to the `litecoin`
//! crate (a rust-bitcoin fork with MWEB support) under the `litecoin` feature.
//!
//! Cargo unifies features per build, so enabling `litecoin` anywhere switches
//! every crate. Crates with Litecoin-only code must therefore forward their own
//! `litecoin` feature here and to every dependency that also has one.

#[cfg(not(feature = "litecoin"))]
pub use bitcoin_crate::*;

#[cfg(feature = "litecoin")]
pub use litecoin::*;
