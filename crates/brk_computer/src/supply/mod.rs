pub mod burned;
pub mod velocity;

mod compute;
mod import;
mod import_sources;
mod vecs;

pub(crate) use import_sources::ImportSources;
pub use vecs::Vecs;

pub const DB_NAME: &str = "supply";
