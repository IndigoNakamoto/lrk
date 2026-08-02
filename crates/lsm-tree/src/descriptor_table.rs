// Copyright (c) 2025-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::GlobalTableId;
use quick_cache::{UnitWeighter, sync::Cache as QuickCache};
use std::{fs::File, sync::Arc};

type Item = Arc<File>;

/// Caches file descriptors to tables
pub struct DescriptorTable {
    inner: QuickCache<GlobalTableId, Item, UnitWeighter, rustc_hash::FxBuildHasher>,
}

impl DescriptorTable {
    #[must_use]
    pub fn new(capacity: usize) -> Self {
        use quick_cache::sync::DefaultLifecycle;

        let quick_cache = QuickCache::with(
            1_000,
            capacity as u64,
            UnitWeighter,
            rustc_hash::FxBuildHasher,
            DefaultLifecycle::default(),
        );

        Self { inner: quick_cache }
    }

    pub(crate) fn len(&self) -> usize {
        self.inner.len()
    }

    #[must_use]
    pub fn access_for_table(&self, id: &GlobalTableId) -> Option<Arc<File>> {
        self.inner.get(id)
    }

    pub fn insert_for_table(&self, id: GlobalTableId, item: Item) {
        self.inner.insert(id, item);
    }

    pub fn remove_for_table(&self, id: &GlobalTableId) {
        self.inner.remove(id);
    }
}
