// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    SeqNo, SequenceNumberCounter,
    memtable::Memtable,
    tree::sealed::SealedMemtables,
    version::{Version, persist_version},
};
use arc_swap::ArcSwap;
use std::{collections::VecDeque, path::Path, sync::Arc};

/// A super version is a point-in-time snapshot of memtables and a [`Version`] (list of disk files)
#[derive(Clone)]
pub struct SuperVersion {
    /// Active memtable that is being written to
    #[doc(hidden)]
    pub active_memtable: Arc<Memtable>,

    /// Frozen memtables that are being flushed
    pub(crate) sealed_memtables: Arc<SealedMemtables>,

    /// Current tree version
    pub(crate) version: Version,

    pub(crate) seqno: SeqNo,
}

pub struct SuperVersions {
    versions: VecDeque<Arc<SuperVersion>>,
    latest: Arc<ArcSwap<SuperVersion>>,
}

impl SuperVersions {
    pub fn new(version: Version) -> Self {
        let version = Arc::new(SuperVersion {
            active_memtable: Arc::new(Memtable::new(0)),
            sealed_memtables: Arc::default(),
            version,
            seqno: 0,
        });

        Self {
            versions: vec![version.clone()].into(),
            latest: Arc::new(ArcSwap::from(version)),
        }
    }

    pub fn memtable_size_sum(&self) -> u64 {
        let mut set = crate::HashMap::default();

        for super_version in &self.versions {
            set.entry(super_version.active_memtable.id)
                .and_modify(|bytes| *bytes += super_version.active_memtable.size())
                .or_insert_with(|| super_version.active_memtable.size());

            for sealed in super_version.sealed_memtables.iter() {
                set.entry(sealed.id)
                    .and_modify(|bytes| *bytes += sealed.size())
                    .or_insert_with(|| sealed.size());
            }
        }

        set.into_values().sum()
    }

    pub fn len(&self) -> usize {
        self.versions.len()
    }

    pub fn free_list_len(&self) -> usize {
        self.len().saturating_sub(1)
    }

    pub fn maintenance(&mut self, folder: &Path, gc_watermark: SeqNo) -> crate::Result<()> {
        if gc_watermark == 0 {
            return Ok(());
        }

        if self.free_list_len() < 1 {
            return Ok(());
        }

        log::trace!("Running manifest GC with watermark={gc_watermark}");

        if let Some(hi_idx) = self.versions.iter().rposition(|x| x.seqno < gc_watermark) {
            for _ in 0..hi_idx {
                let Some(head) = self.versions.front() else {
                    break;
                };

                log::trace!(
                    "Removing version #{} (seqno={})",
                    head.version.id(),
                    head.seqno,
                );

                let path = folder.join(format!("v{}", head.version.id()));
                if path.try_exists()? {
                    crate::file::retry_transient_io(|| std::fs::remove_file(&path))?;
                }

                self.versions.pop_front();
            }
        }

        log::trace!(
            "Manifest GC done, version length now {}",
            self.versions.len()
        );

        Ok(())
    }

    /// Modifies the level manifest atomically.
    ///
    /// The function accepts a transition function that receives the current version
    /// and returns a new version.
    ///
    /// The function takes care of persisting the version changes on disk.
    pub(crate) fn upgrade_version<F: FnOnce(&SuperVersion) -> crate::Result<SuperVersion>>(
        &mut self,
        tree_path: &Path,
        f: F,
        seqno: &SequenceNumberCounter,
        visible_seqno: &SequenceNumberCounter,
    ) -> crate::Result<()> {
        self.upgrade_version_with_seqno(tree_path, f, seqno.next(), visible_seqno)
    }

    /// Like `upgrade_version`, but takes an already-allocated sequence number.
    ///
    /// This is useful when the seqno must be coordinated with other operations
    /// (e.g., bulk ingestion where tables are recovered with the same seqno).
    pub(crate) fn upgrade_version_with_seqno<
        F: FnOnce(&SuperVersion) -> crate::Result<SuperVersion>,
    >(
        &mut self,
        tree_path: &Path,
        f: F,
        seqno: SeqNo,
        visible_seqno: &SequenceNumberCounter,
    ) -> crate::Result<()> {
        let mut next_version = f(&self.latest_version())?;
        next_version.seqno = seqno;
        log::trace!("Next version seqno={}", next_version.seqno);

        persist_version(tree_path, &next_version.version)?;
        self.append_version(next_version);

        visible_seqno.fetch_max(seqno + 1);

        Ok(())
    }

    pub fn append_version(&mut self, version: SuperVersion) {
        let version = Arc::new(version);
        self.versions.push_back(version.clone());
        self.latest.store(version);
    }

    pub fn replace_latest_version(&mut self, version: SuperVersion) {
        if let Some(latest) = self.versions.back_mut() {
            let version = Arc::new(version);
            *latest = version.clone();
            self.latest.store(version);
        }
    }

    pub fn latest_version(&self) -> SuperVersion {
        #[expect(clippy::expect_used, reason = "SuperVersion is expected to exist")]
        self.versions
            .back()
            .map(|version| version.as_ref().clone())
            .expect("should always have a SuperVersion")
    }

    pub(crate) fn latest_version_reader(&self) -> Arc<ArcSwap<SuperVersion>> {
        self.latest.clone()
    }

    pub(crate) fn get_version_arc_for_snapshot(&self, seqno: SeqNo) -> Arc<SuperVersion> {
        if seqno == 0 {
            #[expect(clippy::expect_used, reason = "SuperVersion is expected to exist")]
            return self
                .versions
                .front()
                .cloned()
                .expect("should always find a SuperVersion");
        }

        let version = self
            .versions
            .iter()
            .rev()
            .find(|version| version.seqno < seqno)
            .cloned();

        if version.is_none() {
            log::error!("Failed to find a SuperVersion for snapshot with seqno={seqno}");
            log::error!("SuperVersions:");

            for version in self.versions.iter().rev() {
                log::error!("-> {}, seqno={}", version.version.id(), version.seqno);
            }
        }

        #[expect(clippy::expect_used, reason = "SuperVersion is expected to exist")]
        version.expect("should always find a SuperVersion")
    }

    pub fn get_version_for_snapshot(&self, seqno: SeqNo) -> SuperVersion {
        self.get_version_arc_for_snapshot(seqno).as_ref().clone()
    }

    #[cfg(test)]
    fn from_versions(versions: VecDeque<SuperVersion>) -> Self {
        let versions: VecDeque<_> = versions.into_iter().map(Arc::new).collect();
        #[expect(clippy::expect_used, reason = "test histories are non-empty")]
        let latest = versions
            .back()
            .cloned()
            .expect("history should not be empty");

        Self {
            versions,
            latest: Arc::new(ArcSwap::from(latest)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn latest_reader_tracks_publications() {
        let mut history = SuperVersions::new(Version::new(0));
        let reader = history.latest_version_reader();
        let original = reader.load_full();

        let mut appended = history.latest_version();
        appended.version = Version::new(1);
        history.append_version(appended);

        assert_eq!(original.version.id(), 0);
        assert_eq!(reader.load().version.id(), 1);

        let mut replacement = history.latest_version();
        replacement.version = Version::new(2);
        history.replace_latest_version(replacement);

        assert_eq!(reader.load().version.id(), 2);
    }

    #[test]
    fn super_version_gc_above_watermark() -> crate::Result<()> {
        let mut history = SuperVersions::from_versions(
            vec![
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 0,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 1,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 2,
                },
            ]
            .into(),
        );

        history.maintenance(Path::new("."), 0)?;

        assert_eq!(history.free_list_len(), 2);

        Ok(())
    }

    #[test]
    fn super_version_gc_below_watermark_simple() -> crate::Result<()> {
        let mut history = SuperVersions::from_versions(
            vec![
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 0,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 1,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 2,
                },
            ]
            .into(),
        );

        history.maintenance(Path::new("."), 3)?;

        assert_eq!(history.len(), 1);

        Ok(())
    }

    #[test]
    fn super_version_gc_below_watermark_simple_2() -> crate::Result<()> {
        let mut history = SuperVersions::from_versions(
            vec![
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 0,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 1,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 2,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 8,
                },
            ]
            .into(),
        );

        history.maintenance(Path::new("."), 3)?;

        assert_eq!(history.len(), 2);

        Ok(())
    }

    #[test]
    fn super_version_gc_below_watermark_keep() -> crate::Result<()> {
        let mut history = SuperVersions::from_versions(
            vec![
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 0,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 8,
                },
            ]
            .into(),
        );

        history.maintenance(Path::new("."), 3)?;

        assert_eq!(history.len(), 2);

        Ok(())
    }

    #[test]
    fn super_version_gc_below_watermark_shadowed() -> crate::Result<()> {
        let mut history = SuperVersions::from_versions(
            vec![
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 0,
                },
                SuperVersion {
                    active_memtable: Arc::new(Memtable::new(0)),
                    sealed_memtables: Arc::default(),
                    version: Version::new(0),
                    seqno: 2,
                },
            ]
            .into(),
        );

        history.maintenance(Path::new("."), 3)?;

        assert_eq!(history.len(), 1);

        Ok(())
    }
}
