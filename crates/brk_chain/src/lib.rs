//! Chain-agnostic primitives re-export and chain constants for BRK.
//!
//! Enable the `litecoin` Cargo feature (and disable the default `bitcoin`
//! feature) to compile BRK against the Litecoin network:
//!
//! ```toml
//! brk_chain = { ..., default-features = false, features = ["litecoin"] }
//! ```
//!
//! All downstream crates that depend on `brk_chain` then access protocol types
//! through `brk_chain::primitives`, which resolves to either the `bitcoin` or
//! `litecoin` crate depending on the active feature.

mod chain;
pub use chain::{Chain, ChainConstants};

// Re-export the active chain's primitive types under a stable module path so
// downstream crates can write `use brk_chain::primitives as bitcoin` and keep
// all existing `bitcoin::Foo` references working without modification.

#[cfg(all(feature = "bitcoin", not(feature = "litecoin")))]
pub use ::bitcoin as primitives;

#[cfg(feature = "litecoin")]
pub use ::litecoin as primitives;
