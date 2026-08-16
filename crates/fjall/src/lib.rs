// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

//! Fjall is a log-structured embeddable key-value storage engine written in Rust. It features:
//!
//! - A thread-safe BTreeMap-like API
//! - 100% safe & stable Rust
//! - LSM-tree-based storage similar to `RocksDB`
//! - Range & prefix searching with forward and reverse iteration
//! - Keyspaces (a.k.a. column families) with cross-keyspace atomic semantics
//! - Built-in compression (default = `LZ4`)
//! - Serializable transactions (optional)
//! - Automatic background maintenance
//!
//! It is not:
//!
//! - A standalone database server
//! - A relational or wide-column database: it has no notion of columns or query language
//!
//! Keys are limited to 65536 bytes, values are limited to 2^32 bytes. As is normal with any kind of storage engine, larger keys and values have a bigger performance impact.
//!
//! For the underlying LSM-tree implementation, see: <https://crates.io/crates/lsm-tree>.
//!
//! ## Basic usage
//!
//! ```
//! use fjall::{PersistMode, Database, KeyspaceCreateOptions};
//! #
//! # let folder = tempfile::tempdir().unwrap();
//!
//! // A database may contain multiple keyspaces
//! // You should probably only use a single database for your application
//! let db = Database::builder(&folder).open()?;
//! // TxDatabase::builder for transactional semantics
//!
//! // Each keyspace is its own physical LSM-tree
//! let items = db.keyspace("my_items", KeyspaceCreateOptions::default)?;
//!
//! // Write some data
//! items.insert("a", "hello")?;
//!
//! // And retrieve it
//! let bytes = items.get("a")?;
//!
//! // Or remove it again
//! items.remove("a")?;
//!
//! // Search by prefix
//! for kv in items.prefix("user1") {
//!   // ...
//! }
//!
//! // Iterators implement DoubleEndedIterator, so you can search backwards, too!
//! for kv in items.prefix("prefix").rev() {
//!   // ...
//! }
//!
//! // Search by range
//! for kv in items.range("a"..="z") {
//!   // ...
//! }
//!
//! // Sync the journal to disk to make sure data is definitely durable
//! // When the database is dropped, it will try to persist with `PersistMode::SyncAll` as well
//! db.persist(PersistMode::SyncAll)?;
//! #
//! # Ok::<_, fjall::Error>(())
//! ```

#![doc(html_logo_url = "https://raw.githubusercontent.com/fjall-rs/fjall/main/logo.png")]
#![doc(html_favicon_url = "https://raw.githubusercontent.com/fjall-rs/fjall/main/logo.png")]
#![deny(unsafe_code)]
#![deny(clippy::all, missing_docs, clippy::cargo)]
#![allow(
    clippy::cargo_common_metadata,
    reason = "not every internal workspace package is independently published"
)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::indexing_slicing)]
#![warn(clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::expect_used,
    clippy::missing_panics_doc,
    reason = "poisoned locks and violated persisted-data invariants are unrecoverable"
)]
#![allow(clippy::missing_const_for_fn, clippy::significant_drop_tightening)]
#![allow(
    clippy::multiple_crate_versions,
    reason = "transitive dependencies currently require distinct hashbrown versions"
)]
#![cfg_attr(
    test,
    allow(
        clippy::items_after_statements,
        clippy::unwrap_used,
        reason = "test fixtures favor direct assertions and local helper imports"
    )
)]
#![cfg_attr(docsrs, feature(doc_cfg))]

macro_rules! fail_iter {
    ($e:expr) => {
        match $e {
            Ok(v) => v,
            Err(e) => return Some(Err(e.into())),
        }
    };
}

mod batch;
mod builder;

/// Contains compaction strategies
pub mod compaction;

mod db_config;

mod db;

#[cfg(test)]
mod db_test;

mod error;
mod file;
mod flush;
mod guard;
mod ingestion;
mod iter;
mod journal;
mod keyspace;
mod locked_file;
mod meta_keyspace;
mod path;
mod poison;
mod readable;
mod recovery;
mod snapshot;
mod snapshot_nonce;
mod snapshot_tracker;
mod stats;
mod supervisor;
mod version;
mod worker_pool;
mod write_buffer_manager;

pub(crate) type HashMap<K, V> =
    std::collections::HashMap<K, V, xxhash_rust::xxh3::Xxh3DefaultBuilder>;

/// Configuration policies
pub mod config {
    pub use lsm_tree::config::{
        BlockSizePolicy, BloomConstructionPolicy, CompressionPolicy, FilterPolicy,
        FilterPolicyEntry, HashRatioPolicy, PartitioningPolicy, PinningPolicy,
        RestartIntervalPolicy,
    };
}

pub use {
    batch::WriteBatch as OwnedWriteBatch,
    builder::Builder as DatabaseBuilder,
    db::Database,
    db_config::Config,
    error::{Error, Result},
    guard::Guard,
    iter::Iter,
    journal::{error::RecoveryError as JournalRecoveryError, writer::PersistMode},
    keyspace::{Keyspace, options::CreateOptions as KeyspaceCreateOptions},
    readable::Readable,
    snapshot::Snapshot,
    version::FormatVersion,
};

#[doc(hidden)]
pub use lsm_tree::{AbstractTree, Error as LsmError};

pub use lsm_tree::{CompressionType, KvPair, SeqNo, Slice, UserKey, UserValue};

/// Utility functions
pub mod util {
    pub use lsm_tree::util::{prefix_to_range, prefixed_range};
}
