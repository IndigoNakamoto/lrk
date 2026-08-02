// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use super::Tree;
use crate::{UserKey, UserValue, config::FilterPolicyEntry, table::multi_writer::MultiWriter};

pub const INITIAL_CANONICAL_LEVEL: usize = 1;

/// Bulk ingestion
///
/// Items NEED to be added in ascending key order.
///
/// Ingested data bypasses memtables and is written directly into new tables,
/// using the same table writer configuration that is used for flush and compaction.
pub struct Ingestion<'a> {
    tree: &'a Tree,
    pub(crate) writer: MultiWriter,
    #[cfg(debug_assertions)]
    last_key: Option<UserKey>,
}

impl<'a> Ingestion<'a> {
    /// Creates a new ingestion.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn new(tree: &'a Tree) -> crate::Result<Self> {
        let folder = tree.config.path.join(crate::file::TABLES_FOLDER);
        log::debug!("Ingesting into tables in {}", folder.display());

        let index_partitioning = tree
            .config
            .index_block_partitioning_policy
            .get(INITIAL_CANONICAL_LEVEL);

        let filter_partitioning = tree
            .config
            .filter_block_partitioning_policy
            .get(INITIAL_CANONICAL_LEVEL);

        // TODO: maybe create a PrepareMultiWriter that can be used by flush, ingest and compaction worker
        let mut writer =
            MultiWriter::new(folder, tree.table_id_counter.clone(), 64 * 1_024 * 1_024, 6)?
                .use_bloom_policy({
                    if tree.config.expect_point_read_hits {
                        crate::config::BloomConstructionPolicy::BitsPerKey(0.0)
                    } else if let FilterPolicyEntry::Bloom(p) =
                        tree.config.filter_policy.get(INITIAL_CANONICAL_LEVEL)
                    {
                        p
                    } else {
                        crate::config::BloomConstructionPolicy::BitsPerKey(0.0)
                    }
                })
                .use_data_block_size(
                    tree.config
                        .data_block_size_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                )
                .use_data_block_hash_ratio(
                    tree.config
                        .data_block_hash_ratio_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                )
                .use_data_block_compression(
                    tree.config
                        .data_block_compression_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                )
                .use_index_block_compression(
                    tree.config
                        .index_block_compression_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                )
                .use_data_block_restart_interval(
                    tree.config
                        .data_block_restart_interval_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                )
                .use_index_block_restart_interval(
                    tree.config
                        .index_block_restart_interval_policy
                        .get(INITIAL_CANONICAL_LEVEL),
                );

        if index_partitioning {
            writer = writer.use_partitioned_index();
        }
        if filter_partitioning {
            writer = writer.use_partitioned_filter();
        }

        Ok(Self {
            tree,
            writer,
            #[cfg(debug_assertions)]
            last_key: None,
        })
    }

    #[cfg(debug_assertions)]
    fn validate_key(&mut self, key: &UserKey) {
        if let Some(previous) = &self.last_key {
            debug_assert!(
                key > previous,
                "next key in ingestion must be greater than last key"
            );
        }
        self.last_key = Some(key.clone());
    }

    /// Writes a key-value pair.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn write<K: Into<UserKey>, V: Into<UserValue>>(
        &mut self,
        key: K,
        value: V,
    ) -> crate::Result<()> {
        let key = key.into();
        let value = value.into();

        #[cfg(debug_assertions)]
        self.validate_key(&key);

        self.writer
            .write_distinct(crate::InternalValue::from_components(
                key,
                value,
                0,
                crate::ValueType::Value,
            ))
    }

    /// Writes a tombstone for a key.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn write_tombstone<K: Into<UserKey>>(&mut self, key: K) -> crate::Result<()> {
        let key = key.into();

        #[cfg(debug_assertions)]
        self.validate_key(&key);

        self.writer
            .write_distinct(crate::InternalValue::from_components(
                key,
                crate::UserValue::empty(),
                0,
                crate::ValueType::Tombstone,
            ))
    }

    /// Writes a weak tombstone for a key.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn write_weak_tombstone<K: Into<UserKey>>(&mut self, key: K) -> crate::Result<()> {
        let key = key.into();

        #[cfg(debug_assertions)]
        self.validate_key(&key);

        self.writer
            .write_distinct(crate::InternalValue::from_components(
                key,
                crate::UserValue::empty(),
                0,
                crate::ValueType::WeakTombstone,
            ))
    }

    /// Finishes the ingestion.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    #[allow(clippy::significant_drop_tightening)]
    pub fn finish(self) -> crate::Result<()> {
        self.finish_inner(true)
    }

    /// Finishes ingestion without checking or flushing memtables.
    ///
    /// The caller must ensure that no memtable writes can occur before or
    /// during completion.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    #[allow(clippy::significant_drop_tightening)]
    pub fn finish_exclusive(self) -> crate::Result<()> {
        #[cfg(debug_assertions)]
        {
            #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
            let super_version = self
                .tree
                .version_history
                .read()
                .expect("lock is poisoned")
                .latest_version();

            debug_assert!(
                super_version.active_memtable.is_empty()
                    && super_version
                        .sealed_memtables
                        .iter()
                        .all(|memtable| memtable.is_empty()),
                "exclusive ingestion requires empty memtables",
            );
        }

        self.finish_inner(false)
    }

    fn finish_inner(self, flush_memtables: bool) -> crate::Result<()> {
        use crate::{AbstractTree, Table};

        if self.writer.is_empty() {
            log::trace!("No data written to Ingestion, returning early");
            return Ok(());
        }

        // General path critical section: atomic flush + seqno allocation +
        // registration. The exclusive path skips this because its caller
        // guarantees that memtable writes cannot occur.
        //
        // We must ensure no concurrent writes interfere between flushing the
        // active memtable and registering the ingested tables. The sequence is:
        //   1. Acquire flush lock (prevents concurrent flushes)
        //   2. Flush active memtable (ensures no pending writes)
        //   3. Finish ingestion writer (creates table files)
        //   4. Allocate next global seqno (atomic timestamp)
        //   5. Recover tables with that seqno
        //   6. Register version with same seqno
        //
        // Why not flush in new()?
        // If we flushed in new(), there would be a race condition:
        //   new() -> flush -> [TIME PASSES + OTHER WRITES] -> finish() -> seqno
        // The seqno would be disconnected from the flush, violating MVCC.
        //
        // By holding the flush lock throughout, we guarantee atomicity.
        let flush_lock = flush_memtables.then(|| self.tree.get_flush_lock());

        if let Some(flush_lock) = flush_lock.as_ref() {
            // Flush any pending memtable writes to ensure ingestion sees a
            // consistent snapshot and lookup order remains correct.
            // We call rotate + flush directly because we already hold the lock.
            self.tree.rotate_memtable();
            self.tree.flush(flush_lock, 0)?;
        }

        // Finalize the ingestion writer, writing all buffered data to disk.
        let (folder, results) = self.writer.finish()?;

        log::info!("Finished ingestion writer");

        // Acquire locks for version registration. We must hold both the
        // compaction state lock and version history lock to safely modify
        // the tree's version.
        #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
        let mut _compaction_state = self.tree.compaction_state.lock().expect("lock is poisoned");

        #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
        let mut version_lock = self.tree.version_history.write().expect("lock is poisoned");

        // Allocate the next global sequence number. This seqno will be shared
        // by all ingested tables and the version that registers them, ensuring
        // consistent MVCC snapshots.
        let global_seqno = self.tree.config.seqno.next();

        // Recover all created tables, assigning them the global_seqno we just
        // allocated. This ensures all ingested tables share the same sequence
        // number, which is critical for MVCC correctness.
        //
        // We intentionally do NOT pin filter/index blocks here. Large ingests
        // are typically placed in level 1, and pinning would increase memory
        // pressure unnecessarily.
        let created_tables = results
            .into_iter()
            .map(|(table_id, checksum)| -> crate::Result<Table> {
                Table::recover(
                    folder.join(table_id.to_string()),
                    checksum,
                    global_seqno,
                    self.tree.id,
                    self.tree.config.cache.clone(),
                    self.tree.config.descriptor_table.clone(),
                    false,
                    false,
                )
            })
            .collect::<crate::Result<Vec<_>>>()?;

        // Upgrade the version with our ingested tables, using the global_seqno
        // we allocated earlier. This ensures the version and all tables share
        // the same sequence number.
        //
        // We use upgrade_version_with_seqno (instead of upgrade_version) because
        // we need precise control over the seqno: it must match the seqno we
        // already assigned to the recovered tables.
        version_lock.upgrade_version_with_seqno(
            &self.tree.config.path,
            |current| {
                let mut copy = current.clone();
                copy.version = copy.version.with_new_l0_run(&created_tables);
                Ok(copy)
            },
            global_seqno,
            &self.tree.config.visible_seqno,
        )?;

        // Perform maintenance on the version history (e.g., clean up old versions).
        // We use gc_watermark=0 since ingestion doesn't affect sealed memtables.
        if let Err(e) = version_lock.maintenance(&self.tree.config.path, 0) {
            log::warn!("Version GC failed: {e:?}");
        }

        Ok(())
    }
}

impl Tree {
    /// Starts a bulk ingestion into this tree.
    ///
    /// # Errors
    ///
    /// Will return `Err` if an IO error occurs.
    pub fn ingestion(&self) -> crate::Result<Ingestion<'_>> {
        Ingestion::new(self)
    }
}
