// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{SequenceNumberCounter, tree::inner::TreeId};

pub type TableId = u32;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct GlobalTableId(TreeId, TableId);

impl GlobalTableId {
    #[must_use]
    pub fn tree_id(&self) -> TreeId {
        self.0
    }

    #[must_use]
    pub fn table_id(&self) -> TableId {
        self.1
    }
}

impl From<(TreeId, TableId)> for GlobalTableId {
    fn from((tree_id, table_id): (TreeId, TableId)) -> Self {
        Self(tree_id, table_id)
    }
}

#[expect(
    clippy::expect_used,
    reason = "exhausting the complete u32 table ID space is unrecoverable"
)]
pub fn next_table_id(counter: &SequenceNumberCounter) -> TableId {
    counter.next().try_into().expect("ran out of table IDs")
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    fn global_table_id_accessors() {
        let tree_id = 42;
        let table_id: TableId = 7;
        let global_table_id = GlobalTableId::from((tree_id, table_id));

        assert_eq!(global_table_id.tree_id(), 42);
        assert_eq!(global_table_id.table_id(), 7);
        assert_eq!(size_of::<GlobalTableId>(), 8);
    }
}
