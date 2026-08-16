// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    config::{
        BlockSizePolicy, BloomConstructionPolicy, CompressionPolicy, FilterPolicy,
        FilterPolicyEntry, HashRatioPolicy, PartitioningPolicy, PinningPolicy,
        RestartIntervalPolicy,
    },
    keyspace::{InternalKeyspaceId, config::DecodeConfig},
    meta_keyspace::{MetaKeyspace, encode_config_key},
};
use byteorder::ReadBytesExt;
use lsm_tree::{CompressionType, KvPair, compaction::Factory as CompactionFilterFactory};
use std::sync::Arc;

/// Options to configure a keyspace
#[expect(clippy::module_name_repetitions)]
#[derive(Clone)]
pub struct CreateOptions {
    /// Number of levels of the LSM tree (depth of tree).
    pub(crate) level_count: u8,

    /// Maximum size of this keyspace's memtable - can be changed during runtime
    pub(crate) max_memtable_size: u64,

    /// Data block hash ratio
    #[doc(hidden)]
    pub data_block_hash_ratio_policy: HashRatioPolicy,

    /// Block size of data blocks.
    #[doc(hidden)]
    pub data_block_size_policy: BlockSizePolicy,

    #[doc(hidden)]
    pub data_block_restart_interval_policy: RestartIntervalPolicy,

    #[doc(hidden)]
    pub index_block_restart_interval_policy: RestartIntervalPolicy,

    #[doc(hidden)]
    pub index_block_pinning_policy: PinningPolicy,

    #[doc(hidden)]
    pub filter_block_pinning_policy: PinningPolicy,

    #[doc(hidden)]
    pub filter_block_partitioning_policy: PartitioningPolicy,

    #[doc(hidden)]
    pub index_block_partitioning_policy: PartitioningPolicy,

    /// If `true`, the last level will not build filters, reducing the filter size of a database
    /// by ~90% typically.
    #[doc(hidden)]
    pub expect_point_read_hits: bool,

    /// Filter construction policy.
    #[doc(hidden)]
    pub filter_policy: FilterPolicy,

    /// Compression to use for data blocks.
    #[doc(hidden)]
    pub data_block_compression_policy: CompressionPolicy,

    /// Compression to use for index blocks.
    #[doc(hidden)]
    pub index_block_compression_policy: CompressionPolicy,

    pub(crate) manual_journal_persist: bool,

    pub(crate) compaction_filter_factory: Option<Arc<dyn CompactionFilterFactory>>,
}

impl Default for CreateOptions {
    fn default() -> Self {
        let default_tree_config = lsm_tree::Config::default();

        Self {
            manual_journal_persist: false,

            max_memtable_size: /* 64 MiB */ 64 * 1_024 * 1_024,

            data_block_hash_ratio_policy: HashRatioPolicy::all(0.0),

            data_block_size_policy: BlockSizePolicy::all(/* 4 KiB */ 4 * 1_024),

            data_block_restart_interval_policy:  RestartIntervalPolicy::new([10, 16]),
            index_block_restart_interval_policy:  RestartIntervalPolicy::all(1),

            index_block_pinning_policy: PinningPolicy::new([true, true, false]),
            filter_block_pinning_policy: PinningPolicy::new([true, false]),

            index_block_partitioning_policy: PartitioningPolicy::new([false, false, false, true]),
            filter_block_partitioning_policy: PartitioningPolicy::new([false, false, false, true]),

            expect_point_read_hits: false,

            filter_policy: FilterPolicy::new([
                FilterPolicyEntry::Bloom(BloomConstructionPolicy::FalsePositiveRate(0.0001)),
                FilterPolicyEntry::Bloom(BloomConstructionPolicy::BitsPerKey(10.0)),
            ]),

            level_count: default_tree_config.level_count,

            #[cfg(feature = "lz4")]
            data_block_compression_policy: CompressionPolicy::new([CompressionType::None, CompressionType::None, CompressionType::Lz4]),

            #[cfg(not(feature = "lz4"))]
            data_block_compression_policy: CompressionPolicy::new(&[CompressionType::None]),

            index_block_compression_policy: CompressionPolicy::all(CompressionType::None),

            compaction_filter_factory: None,
        }
    }
}

macro_rules! policy {
    ($keyspace_id:expr, $name:expr, $field:expr) => {{
        let key = encode_config_key($keyspace_id, $name);
        (key.into(), $field.encode())
    }};
}

impl CreateOptions {
    /// Installs a compaction filter factory.
    pub(crate) fn with_compaction_filter_factory(
        mut self,
        factory: Arc<dyn CompactionFilterFactory + 'static>,
    ) -> Self {
        self.compaction_filter_factory = Some(factory);
        self
    }

    #[expect(clippy::expect_used)]
    pub(crate) fn from_kvs(
        keyspace_id: InternalKeyspaceId,
        meta_keyspace: &MetaKeyspace,
    ) -> crate::Result<Self> {
        let data_block_compression_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "data_block_compression_policy")?
            .expect("should exist");
        let data_block_compression_policy =
            CompressionPolicy::decode(&data_block_compression_policy)?;

        let index_block_compression_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "index_block_compression_policy")?
            .expect("should exist");
        let index_block_compression_policy =
            CompressionPolicy::decode(&index_block_compression_policy)?;

        let data_block_size_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "data_block_size_policy")?
            .expect("should exist");
        let data_block_size_policy = BlockSizePolicy::decode(&data_block_size_policy)?;

        let filter_block_partitioning_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "filter_block_partitioning_policy")?
            .expect("should exist");
        let filter_block_partitioning_policy =
            PinningPolicy::decode(&filter_block_partitioning_policy)?;

        let index_block_partitioning_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "index_block_partitioning_policy")?
            .expect("should exist");
        let index_block_partitioning_policy =
            PinningPolicy::decode(&index_block_partitioning_policy)?;

        let filter_block_pinning_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "filter_block_pinning_policy")?
            .expect("should exist");
        let filter_block_pinning_policy = PinningPolicy::decode(&filter_block_pinning_policy)?;

        let index_block_pinning_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "index_block_pinning_policy")?
            .expect("should exist");
        let index_block_pinning_policy = PinningPolicy::decode(&index_block_pinning_policy)?;

        let data_block_restart_interval_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "data_block_restart_interval_policy")?
            .expect("should exist");
        let data_block_restart_interval_policy =
            RestartIntervalPolicy::decode(&data_block_restart_interval_policy)?;

        let index_block_restart_interval_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "index_block_restart_interval_policy")?
            .expect("should exist");
        let index_block_restart_interval_policy =
            RestartIntervalPolicy::decode(&index_block_restart_interval_policy)?;

        let data_block_hash_ratio_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "data_block_hash_ratio_policy")?
            .expect("should exist");
        let data_block_hash_ratio_policy = HashRatioPolicy::decode(&data_block_hash_ratio_policy)?;

        let expect_point_read_hits = meta_keyspace
            .get_kv_for_config(keyspace_id, "expect_point_read_hits")?
            .expect("should exist");
        let expect_point_read_hits = expect_point_read_hits == [1];

        let filter_policy = meta_keyspace
            .get_kv_for_config(keyspace_id, "filter_policy")?
            .expect("should exist");
        let filter_policy = FilterPolicy::decode(&filter_policy)?;

        let manual_journal_persist = meta_keyspace
            .get_kv_for_config(keyspace_id, "manual_journal_persist")?
            .expect("should exist")
            == [1];

        let max_memtable_size = meta_keyspace
            .get_kv_for_config(keyspace_id, "max_memtable_size")?
            .expect("should exist");
        let max_memtable_size = (&mut &max_memtable_size[..]).read_u64::<byteorder::LE>()?;

        Ok(Self {
            data_block_hash_ratio_policy,

            filter_block_partitioning_policy,
            index_block_partitioning_policy,

            filter_block_pinning_policy,
            index_block_pinning_policy,

            data_block_compression_policy,
            index_block_compression_policy,

            data_block_size_policy,

            data_block_restart_interval_policy,
            index_block_restart_interval_policy,

            expect_point_read_hits,
            filter_policy,

            level_count: 7, // Levels are currently hard coded to 7

            manual_journal_persist,

            max_memtable_size,

            compaction_filter_factory: None,
        })
    }

    pub(crate) fn encode_kvs(&self, keyspace_id: InternalKeyspaceId) -> Vec<KvPair> {
        use crate::keyspace::config::EncodeConfig;

        let kvs = vec![
            policy!(
                keyspace_id,
                "data_block_compression_policy",
                self.data_block_compression_policy
            ),
            policy!(
                keyspace_id,
                "data_block_hash_ratio_policy",
                self.data_block_hash_ratio_policy
            ),
            policy!(
                keyspace_id,
                "data_block_restart_interval_policy",
                self.data_block_restart_interval_policy
            ),
            policy!(
                keyspace_id,
                "data_block_size_policy",
                self.data_block_size_policy
            ),
            {
                let key = encode_config_key(keyspace_id, "expect_point_read_hits");

                let value = (if self.expect_point_read_hits {
                    [1u8]
                } else {
                    [0u8]
                })
                .into();

                (key, value)
            },
            policy!(
                keyspace_id,
                "filter_block_partitioning_policy",
                self.filter_block_partitioning_policy
            ),
            policy!(
                keyspace_id,
                "filter_block_pinning_policy",
                self.filter_block_pinning_policy
            ),
            policy!(keyspace_id, "filter_policy", self.filter_policy),
            policy!(
                keyspace_id,
                "index_block_compression_policy",
                self.index_block_compression_policy
            ),
            policy!(
                keyspace_id,
                "index_block_partitioning_policy",
                self.index_block_partitioning_policy
            ),
            policy!(
                keyspace_id,
                "index_block_pinning_policy",
                self.index_block_pinning_policy
            ),
            policy!(
                keyspace_id,
                "index_block_restart_interval_policy",
                self.index_block_restart_interval_policy
            ),
            {
                let key = encode_config_key(keyspace_id, "level_count");
                (key, [self.level_count].into())
            },
            {
                let key = encode_config_key(keyspace_id, "manual_journal_persist");
                (key, [u8::from(self.manual_journal_persist)].into())
            },
            {
                let key = encode_config_key(keyspace_id, "max_memtable_size");
                (key, self.max_memtable_size.to_le_bytes().into())
            },
            {
                let key = encode_config_key(keyspace_id, "version");
                (key, [3u8].into())
            },
        ];

        kvs
    }

    /// Sets the restart interval inside data blocks.
    ///
    /// A higher restart interval saves space while increasing lookup times
    /// inside data blocks.
    ///
    /// Default = 16
    #[must_use]
    pub fn data_block_restart_interval_policy(mut self, policy: RestartIntervalPolicy) -> Self {
        self.data_block_restart_interval_policy = policy;
        self
    }

    // TODO: not supported yet in lsm-tree
    // /// Sets the restart interval inside index blocks.
    // ///
    // /// A higher restart interval saves space while increasing lookup times
    // /// inside index blocks.
    // ///
    // /// Default = 1
    // #[must_use]
    // #[doc(hidden)]
    // pub fn index_block_restart_interval_policy(mut self, policy: RestartIntervalPolicy) -> Self {
    //     self.index_block_restart_interval_policy = policy;
    //     self
    // }

    /// Sets the pinning policy for filter blocks.
    ///
    /// By default, L0 filter blocks are pinned.
    #[must_use]
    pub fn filter_block_pinning_policy(mut self, policy: PinningPolicy) -> Self {
        self.filter_block_pinning_policy = policy;
        self
    }

    /// Sets the pinning policy for index blocks.
    ///
    /// By default, L0 and L1 index blocks are pinned.
    #[must_use]
    pub fn index_block_pinning_policy(mut self, policy: PinningPolicy) -> Self {
        self.index_block_pinning_policy = policy;
        self
    }

    /// Sets the partitioning policy for filter blocks.
    #[must_use]
    pub fn filter_block_partitioning_policy(mut self, policy: PartitioningPolicy) -> Self {
        self.filter_block_partitioning_policy = policy;
        self
    }

    /// Sets the partitioning policy for index blocks.
    #[must_use]
    pub fn index_block_partitioning_policy(mut self, policy: PartitioningPolicy) -> Self {
        self.index_block_partitioning_policy = policy;
        self
    }

    /// Sets the hash ratio for the hash index in data blocks.
    ///
    /// The hash index speeds up point queries by using an embedded
    /// hash map in data blocks, but uses more space/memory.
    ///
    /// In-memory or heavily cached workloads benefit more from a higher hash ratio.
    ///
    /// If 0.0, the hash index is not constructed.
    #[must_use]
    #[doc(hidden)]
    pub fn data_block_hash_ratio_policy(mut self, policy: HashRatioPolicy) -> Self {
        self.data_block_hash_ratio_policy = policy;
        self
    }

    /// Sets the filter policy for data blocks.
    #[must_use]
    pub fn filter_policy(mut self, policy: FilterPolicy) -> Self {
        self.filter_policy = policy;
        self
    }

    /// If `true`, the last level will not build filters, reducing the filter size of a database
    /// by ~90% typically.
    ///
    /// **Enable this only if you know that point reads generally are expected to find a key-value pair.**
    #[must_use]
    pub fn expect_point_read_hits(mut self, b: bool) -> Self {
        self.expect_point_read_hits = b;
        self
    }

    /// Sets the compression policy for data blocks.
    #[must_use]
    pub fn data_block_compression_policy(mut self, policy: CompressionPolicy) -> Self {
        self.data_block_compression_policy = policy;
        self
    }

    /// Sets the compression policy for index blocks.
    #[must_use]
    pub fn index_block_compression_policy(mut self, policy: CompressionPolicy) -> Self {
        self.index_block_compression_policy = policy;
        self
    }

    /// If `false`, writes will flush data to the operating system.
    ///
    /// Default = false
    ///
    /// Set to `true` to handle persistence manually, e.g. manually using `PersistMode::SyncData`.
    #[must_use]
    pub fn manual_journal_persist(mut self, flag: bool) -> Self {
        self.manual_journal_persist = flag;
        self
    }

    /// Sets the maximum memtable size.
    ///
    /// Default = 64 MiB
    ///
    /// Recommended size 8 - 64 MiB, depending on how much memory
    /// is available.
    ///
    /// Conversely, if `max_memtable_size` is larger than 64 MiB,
    /// it may require increasing the database's `max_write_buffer_size`.
    #[must_use]
    pub fn max_memtable_size(mut self, bytes: u64) -> Self {
        self.max_memtable_size = bytes;
        self
    }

    /// Sets the block size.
    ///
    /// Once set for a keyspace, this property is not considered in the future.
    ///
    /// Default = 4 KiB
    ///
    /// For point read heavy workloads (get) a sensible default is
    /// somewhere between 4 - 8 KiB, depending on the average value size.
    ///
    /// For more space efficiency, block size between 16 - 64 KiB are sensible.
    ///
    /// # Panics
    ///
    /// Panics if the block size is smaller than 1 KiB or larger than 1 MiB.
    #[must_use]
    pub fn data_block_size_policy(mut self, policy: BlockSizePolicy) -> Self {
        self.data_block_size_policy = policy;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test]
    #[cfg(not(feature = "lz4"))]
    fn keyspace_opts_compression_none() {
        let mut c = CreateOptions::default();
        assert_eq!(
            c.data_block_compression_policy,
            CompressionPolicy::disabled(),
        );

        c = c.data_block_compression_policy(CompressionPolicy::disabled());
        assert_eq!(
            c.data_block_compression_policy,
            CompressionPolicy::disabled(),
        );
    }

    #[test]
    #[cfg(feature = "lz4")]
    fn keyspace_opts_compression_default() {
        use CompressionType::{Lz4, None as Uncompressed};

        let mut c = CreateOptions::default();
        assert_eq!(
            c.data_block_compression_policy,
            CompressionPolicy::new([Uncompressed, Uncompressed, Lz4]),
        );

        c = c.data_block_compression_policy(CompressionPolicy::disabled());
        assert_eq!(
            c.data_block_compression_policy,
            CompressionPolicy::disabled(),
        );
    }
}
