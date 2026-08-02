// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

//! Contains compaction strategies

pub mod filter;
mod flavour;
pub(crate) mod leveled;
pub(crate) mod major;
pub(crate) mod pulldown;
pub(crate) mod state;
pub(crate) mod stream;
pub(crate) mod worker;

pub use filter::{CompactionFilter, Factory, ItemAccessor, Verdict};
pub use leveled::Strategy as Leveled;

/// Alias for `Leveled`
pub type Levelled = Leveled;

#[doc(hidden)]
pub use pulldown::Strategy as PullDown;

use crate::{
    HashSet, TableId, compaction::state::CompactionState, config::Config, version::Version,
};

/// Input for compactor
///
/// The compaction strategy chooses which tables to compact and how.
/// That information is given to the compactor.
#[derive(Debug, Eq, PartialEq)]
pub struct Input {
    /// Tables to compact
    pub table_ids: HashSet<TableId>,

    /// Level to put the created tables into
    pub dest_level: u8,

    /// The logical level the tables are part of
    pub canonical_level: u8,

    /// Table target size
    ///
    /// If a table merge reaches the size threshold, a new table is started.
    /// This results in a sorted "run" of tables.
    pub target_size: u64,
}

/// Describes what to do (compact or not)
#[derive(Debug, Eq, PartialEq)]
pub enum Choice {
    /// Just do nothing.
    DoNothing,

    /// Moves tables into another level without rewriting.
    Move(Input),

    /// Compacts some tables into a new level.
    Merge(Input),
}

/// Trait for a compaction strategy
///
/// The strategy receives the levels of the LSM-tree as argument
/// and emits a choice on what to do.
#[expect(clippy::module_name_repetitions)]
pub trait CompactionStrategy {
    /// Gets the compaction strategy name.
    fn get_name(&self) -> &'static str;

    /// Decides on what to do based on the current state of the LSM-tree's levels
    fn choose(&self, version: &Version, config: &Config, state: &CompactionState) -> Choice;
}
