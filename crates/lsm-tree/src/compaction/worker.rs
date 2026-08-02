// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use super::{CompactionStrategy, Input as CompactionPayload};
use crate::{
    Config, InternalValue, SeqNo, SequenceNumberCounter, Table, TableId,
    compaction::{
        Choice,
        filter::{Context, StreamFilterAdapter},
        flavour::StandardCompaction,
        state::CompactionState,
        stream::CompactionStream,
    },
    merge::Merger,
    run_scanner::RunScanner,
    stop_signal::StopSignal,
    tree::inner::TreeId,
    version::{Run, SuperVersions, Version},
};
use std::{
    sync::{Arc, Mutex, MutexGuard, RwLock, RwLockReadGuard},
    time::Instant,
};

pub type CompactionReader<'a> = Box<dyn Iterator<Item = crate::Result<InternalValue>> + 'a>;

/// Compaction options
pub struct Options {
    pub tree_id: TreeId,

    pub global_seqno: SequenceNumberCounter,

    pub visible_seqno: SequenceNumberCounter,

    pub table_id_generator: SequenceNumberCounter,

    /// Configuration of tree.
    pub config: Arc<Config>,

    pub version_history: Arc<RwLock<SuperVersions>>,

    /// Compaction strategy to use.
    pub strategy: Arc<dyn CompactionStrategy>,

    /// Stop signal to interrupt a compaction worker in case
    /// the tree is dropped.
    pub stop_signal: StopSignal,

    /// Evicts items that are older than this seqno (MVCC GC).
    pub mvcc_gc_watermark: u64,

    pub compaction_state: Arc<Mutex<CompactionState>>,
}

impl Options {
    pub fn from_tree(tree: &crate::Tree, strategy: Arc<dyn CompactionStrategy>) -> Self {
        Self {
            global_seqno: tree.config.seqno.clone(),
            visible_seqno: tree.config.visible_seqno.clone(),
            tree_id: tree.id,
            table_id_generator: tree.table_id_counter.clone(),
            config: tree.config.clone(),
            version_history: tree.version_history.clone(),
            stop_signal: tree.stop_signal.clone(),
            strategy,
            mvcc_gc_watermark: 0,

            compaction_state: tree.compaction_state.clone(),
        }
    }
}

/// Runs compaction task.
///
/// This will block until the compactor is fully finished.
pub fn do_compaction(opts: &Options) -> crate::Result<()> {
    #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
    let compaction_state = opts.compaction_state.lock().expect("lock is poisoned");

    #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
    let version_history_lock = opts.version_history.read().expect("lock is poisoned");

    let start = Instant::now();
    log::trace!(
        "Consulting compaction strategy {:?}",
        opts.strategy.get_name(),
    );
    let choice = opts.strategy.choose(
        &version_history_lock.latest_version().version,
        &opts.config,
        &compaction_state,
    );

    log::debug!("Compaction choice: {choice:?} in {:?}", start.elapsed());

    match choice {
        Choice::Merge(payload) => {
            merge_tables(compaction_state, version_history_lock, opts, &payload)
        }
        Choice::Move(payload) => {
            drop(version_history_lock);

            move_tables(&compaction_state, opts, &payload)
        }
        Choice::DoNothing => {
            log::trace!("Compactor chose to do nothing");
            Ok(())
        }
    }
}

fn pick_run_indexes(run: &Run<Table>, to_compact: &[TableId]) -> Option<(usize, usize)> {
    let lo = run
        .iter()
        .position(|table| to_compact.contains(&table.id()))?;

    let hi = run
        .iter()
        .rposition(|table| to_compact.contains(&table.id()))?;

    Some((lo, hi))
}

fn create_compaction_stream<'a>(
    version: &Version,
    to_compact: &[TableId],
    eviction_seqno: SeqNo,
) -> crate::Result<Option<CompactionStream<Merger<CompactionReader<'a>>>>> {
    let mut readers: Vec<CompactionReader<'_>> = vec![];
    let mut found = 0;

    for run in version.iter_levels().flat_map(|lvl| lvl.iter()) {
        if run.len() > 1 {
            let Some((lo, hi)) = pick_run_indexes(run, to_compact) else {
                continue;
            };

            readers.push(Box::new(RunScanner::culled(
                run.clone(),
                (Some(lo), Some(hi)),
            )?));

            found += hi - lo + 1;
        } else {
            for table in run.iter().filter(|x| to_compact.contains(&x.metadata.id)) {
                found += 1;
                readers.push(Box::new(table.scan()?));
            }
        }
    }

    Ok(if found == to_compact.len() {
        Some(CompactionStream::new(Merger::new(readers), eviction_seqno))
    } else {
        None
    })
}

fn move_tables(
    compaction_state: &MutexGuard<'_, CompactionState>,
    opts: &Options,
    payload: &CompactionPayload,
) -> crate::Result<()> {
    #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
    let mut version_history_lock = opts.version_history.write().expect("lock is poisoned");

    // Fail-safe for buggy compaction strategies
    if compaction_state
        .hidden_set()
        .should_decline_compaction(payload.table_ids.iter().copied())
    {
        log::warn!(
            "Compaction task created by {:?} contained hidden tables, declining to run it - please report this at https://github.com/fjall-rs/lsm-tree/issues/new?template=bug_report.md",
            opts.strategy.get_name(),
        );
        return Ok(());
    }

    let table_ids = payload.table_ids.iter().copied().collect::<Vec<_>>();

    version_history_lock.upgrade_version(
        &opts.config.path,
        |current| {
            let mut copy = current.clone();

            copy.version = copy
                .version
                .with_moved(&table_ids, payload.dest_level as usize);

            Ok(copy)
        },
        &opts.global_seqno,
        &opts.visible_seqno,
    )?;

    if let Err(e) = version_history_lock.maintenance(&opts.config.path, opts.mvcc_gc_watermark) {
        log::error!("Manifest maintenance failed: {e:?}");
        return Err(e);
    }

    Ok(())
}

fn hidden_guard<T>(
    payload: &CompactionPayload,
    opts: &Options,
    f: impl FnOnce() -> crate::Result<T>,
) -> crate::Result<T> {
    f().inspect_err(|e| {
        log::error!("Compaction failed: {e:?}");

        // IMPORTANT: We need to show tables again on error
        #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
        let mut compaction_state = opts.compaction_state.lock().expect("lock is poisoned");

        compaction_state
            .hidden_set_mut()
            .show(payload.table_ids.iter().copied());
    })
}

#[expect(clippy::too_many_lines)]
fn merge_tables(
    mut compaction_state: MutexGuard<'_, CompactionState>,
    version_history_lock: RwLockReadGuard<'_, SuperVersions>,
    opts: &Options,
    payload: &CompactionPayload,
) -> crate::Result<()> {
    if opts.stop_signal.is_stopped() {
        log::debug!("Stopping before compaction because of stop signal");
        return Ok(());
    }

    // Fail-safe for buggy compaction strategies
    if compaction_state
        .hidden_set()
        .should_decline_compaction(payload.table_ids.iter().copied())
    {
        log::warn!(
            "Compaction task created by {:?} contained hidden tables, declining to run it - please report this at https://github.com/fjall-rs/lsm-tree/issues/new?template=bug_report.md",
            opts.strategy.get_name(),
        );
        drop(version_history_lock);
        return Ok(());
    }

    let current_super_version = version_history_lock.latest_version();

    let Some(tables) = payload
        .table_ids
        .iter()
        .map(|&id| current_super_version.version.get_table(id).cloned())
        .collect::<Option<Vec<_>>>()
    else {
        log::warn!(
            "Compaction task created by {:?} contained tables not referenced in the level manifest",
            opts.strategy.get_name(),
        );
        return Ok(());
    };

    let Some(mut merge_iter) = create_compaction_stream(
        &current_super_version.version,
        &payload.table_ids.iter().copied().collect::<Vec<_>>(),
        opts.mvcc_gc_watermark,
    )?
    else {
        log::warn!(
            "Compaction task tried to compact tables that do not exist, declining to run it"
        );
        return Ok(());
    };

    let dst_lvl = payload.canonical_level.into();
    let last_level = opts.config.level_count - 1;

    // NOTE: Only evict tombstones when reaching the last level,
    // That way we don't resurrect data beneath the tombstone
    let is_last_level = payload.dest_level == last_level;

    merge_iter = merge_iter
        .evict_tombstones(is_last_level)
        .zero_seqnos(false);

    let filter_ctx = Context { is_last_level };

    // Construct the compaction filter
    let mut compaction_filter = opts.config.compaction_filter_factory.as_ref().map(|f| {
        log::trace!("Installing custom compaction filter {:?}", f.name());
        f.make_filter(&filter_ctx)
    });

    let merge_iter = merge_iter.with_filter(StreamFilterAdapter::new(
        compaction_filter.as_deref_mut(),
        &filter_ctx,
    ));

    let table_writer =
        super::flavour::prepare_table_writer(&current_super_version.version, opts, payload)?;

    let mut compactor = StandardCompaction::new(table_writer, tables);

    drop(version_history_lock);

    {
        compaction_state
            .hidden_set_mut()
            .hide(payload.table_ids.iter().copied());
    }

    // IMPORTANT: Unlock exclusive compaction lock as we are now doing the actual (CPU-intensive) compaction
    drop(compaction_state);

    hidden_guard(payload, opts, || {
        for (idx, item) in merge_iter.enumerate() {
            let item = item?;

            compactor.write(item)?;

            if idx % 1_000_000 == 0 && opts.stop_signal.is_stopped() {
                log::debug!("Stopping amidst compaction because of stop signal");
                return Ok(());
            }
        }

        Ok(())
    })?;

    if let Some(filter) = compaction_filter {
        filter.finish();
    }

    #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
    let mut compaction_state = opts.compaction_state.lock().expect("lock is poisoned");

    log::trace!("Acquiring super version write lock");
    #[expect(clippy::expect_used, reason = "lock is expected to not be poisoned")]
    let mut version_history_lock = opts.version_history.write().expect("lock is poisoned");
    log::trace!("Acquired super version write lock");

    compactor
        .finish(&mut version_history_lock, opts, payload, dst_lvl)
        .inspect_err(|e| {
            // NOTE: We cannot use hidden_guard here because we already locked the compaction state

            log::error!("Compaction failed: {e:?}");

            compaction_state
                .hidden_set_mut()
                .show(payload.table_ids.iter().copied());
        })?;

    compaction_state
        .hidden_set_mut()
        .show(payload.table_ids.iter().copied());

    version_history_lock
        .maintenance(&opts.config.path, opts.mvcc_gc_watermark)
        .inspect_err(|e| {
            log::error!("Manifest maintenance failed: {e:?}");
        })?;

    drop(version_history_lock);
    drop(compaction_state);

    log::trace!("Compaction successful");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_compaction_stream, pick_run_indexes};
    use crate::{AbstractTree, SequenceNumberCounter};
    use test_log::test;

    #[test]
    fn compaction_stream_run_not_found() -> crate::Result<()> {
        let folder = tempfile::tempdir()?;

        let tree = crate::Config::new(
            folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        assert!(create_compaction_stream(&tree.current_version(), &[666], 0)?.is_none());

        Ok(())
    }

    #[test]
    #[expect(clippy::unwrap_used)]
    fn compaction_stream_run() -> crate::Result<()> {
        let folder = tempfile::tempdir()?;

        let tree = crate::Config::new(
            folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("b", "b", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("c", "c", 0);
        tree.flush_active_memtable(0)?;

        assert_eq!(
            Some((0, 2)),
            pick_run_indexes(
                tree.current_version()
                    .level(0)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap(),
                &[0, 1, 2],
            )
        );

        Ok(())
    }

    #[test]
    #[expect(clippy::unwrap_used)]
    fn compaction_stream_run_2() -> crate::Result<()> {
        let folder = tempfile::tempdir()?;

        let tree = crate::Config::new(
            folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("b", "b", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("c", "c", 0);
        tree.flush_active_memtable(0)?;

        assert_eq!(
            Some((0, 0)),
            pick_run_indexes(
                tree.current_version()
                    .level(0)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap(),
                &[0],
            )
        );

        Ok(())
    }

    #[test]
    #[expect(clippy::unwrap_used)]
    fn compaction_stream_run_3() -> crate::Result<()> {
        let folder = tempfile::tempdir()?;

        let tree = crate::Config::new(
            folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("b", "b", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("c", "c", 0);
        tree.flush_active_memtable(0)?;

        assert_eq!(
            Some((2, 2)),
            pick_run_indexes(
                tree.current_version()
                    .level(0)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap(),
                &[2],
            )
        );

        Ok(())
    }

    #[test]
    #[expect(clippy::unwrap_used)]
    fn compaction_stream_run_4() -> crate::Result<()> {
        let folder = tempfile::tempdir()?;

        let tree = crate::Config::new(
            folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("b", "b", 0);
        tree.flush_active_memtable(0)?;

        tree.insert("c", "c", 0);
        tree.flush_active_memtable(0)?;

        assert_eq!(
            None,
            pick_run_indexes(
                tree.current_version()
                    .level(0)
                    .unwrap()
                    .iter()
                    .next()
                    .unwrap(),
                &[4],
            )
        );

        Ok(())
    }
}
