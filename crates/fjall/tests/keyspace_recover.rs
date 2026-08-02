use fjall::config::{
    BlockSizePolicy, FilterPolicy, FilterPolicyEntry, HashRatioPolicy, PinningPolicy,
    RestartIntervalPolicy,
};
use fjall::{AbstractTree, Database, KeyspaceCreateOptions};
use test_log::test;

const ITEM_COUNT: usize = 100;

#[test]
fn reload_keyspace_config() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    let data_block_size = BlockSizePolicy::all(6_666);

    let data_block_interval_policy = RestartIntervalPolicy::all(8);
    // let index_block_interval_policy = RestartIntervalPolicy::all(9);

    let filter_block_pinning_policy = PinningPolicy::new([true, true, true, false]);
    let index_block_pinning_policy = PinningPolicy::new([true, true, false]);

    let filter_block_partitioning_policy = PinningPolicy::new([false, false, true]);
    let index_block_partitioning_policy = PinningPolicy::new([false, false, false, false, true]);

    let filter_policy = FilterPolicy::new([FilterPolicyEntry::Bloom(
        fjall::config::BloomConstructionPolicy::FalsePositiveRate(0.54321),
    )]);

    let data_block_hash_ratio_policy = HashRatioPolicy::all(0.5);

    {
        let db = Database::builder(&folder).open()?;

        let keyspace = db.keyspace("default", || {
            KeyspaceCreateOptions::default()
                .data_block_size_policy(data_block_size.clone())
                .data_block_restart_interval_policy(data_block_interval_policy.clone())
                // .index_block_restart_interval_policy(index_block_policy.clone())
                .filter_block_pinning_policy(filter_block_pinning_policy.clone())
                .index_block_pinning_policy(index_block_pinning_policy.clone())
                .filter_block_partitioning_policy(filter_block_partitioning_policy.clone())
                .index_block_partitioning_policy(index_block_partitioning_policy.clone())
                .expect_point_read_hits(true)
                .filter_policy(filter_policy.clone())
                .data_block_hash_ratio_policy(data_block_hash_ratio_policy.clone())
        })?;

        let real_config = keyspace.tree.tree_config();

        assert_eq!(data_block_size, real_config.data_block_size_policy);
        assert_eq!(
            data_block_interval_policy,
            real_config.data_block_restart_interval_policy,
        );
        // assert_eq!(
        //     index_block_interval_policy,
        //     tree.config.index_block_restart_interval_policy,
        // );
        assert_eq!(
            filter_block_partitioning_policy,
            real_config.filter_block_partitioning_policy,
        );
        assert_eq!(
            filter_block_pinning_policy,
            real_config.filter_block_pinning_policy,
        );
        assert_eq!(
            index_block_partitioning_policy,
            real_config.index_block_partitioning_policy,
        );
        assert_eq!(
            index_block_pinning_policy,
            real_config.index_block_pinning_policy,
        );

        assert_eq!(filter_policy, real_config.filter_policy);

        assert_eq!(
            data_block_hash_ratio_policy,
            real_config.data_block_hash_ratio_policy,
        );

        assert!(keyspace.config.expect_point_read_hits);
    };

    {
        let db = Database::builder(&folder).open()?;
        let keyspace = db.keyspace("default", || unreachable!())?;

        let real_config = keyspace.tree.tree_config();

        assert_eq!(data_block_size, real_config.data_block_size_policy);
        assert_eq!(
            data_block_interval_policy,
            real_config.data_block_restart_interval_policy,
        );
        // assert_eq!(
        //     index_block_interval_policy,
        //     tree.config.index_block_restart_interval_policy,
        // );
        assert_eq!(
            filter_block_partitioning_policy,
            real_config.filter_block_partitioning_policy,
        );
        assert_eq!(
            filter_block_pinning_policy,
            real_config.filter_block_pinning_policy,
        );
        assert_eq!(
            index_block_partitioning_policy,
            real_config.index_block_partitioning_policy,
        );
        assert_eq!(
            index_block_pinning_policy,
            real_config.index_block_pinning_policy,
        );

        assert_eq!(filter_policy, real_config.filter_policy);

        assert_eq!(
            data_block_hash_ratio_policy,
            real_config.data_block_hash_ratio_policy,
        );

        assert!(keyspace.config.expect_point_read_hits);
    }

    Ok(())
}

#[test]
fn reload_with_keyspaces() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    {
        let db = Database::builder(&folder).open()?;

        let keyspaces = &[
            db.keyspace("default1", KeyspaceCreateOptions::default)?,
            db.keyspace("default2", KeyspaceCreateOptions::default)?,
            db.keyspace("default3", KeyspaceCreateOptions::default)?,
        ];

        for tree in keyspaces {
            for x in 0..ITEM_COUNT as u64 {
                let key = x.to_be_bytes();
                let value = nanoid::nanoid!();
                tree.insert(key, value.as_bytes())?;
            }

            for x in 0..ITEM_COUNT as u64 {
                let key: [u8; 8] = (x + ITEM_COUNT as u64).to_be_bytes();
                let value = nanoid::nanoid!();
                tree.insert(key, value.as_bytes())?;
            }
        }

        for tree in keyspaces {
            assert_eq!(tree.len()?, ITEM_COUNT * 2);
            assert_eq!(tree.iter().flat_map(|x| x.key()).count(), ITEM_COUNT * 2);
            assert_eq!(
                tree.iter().rev().flat_map(|x| x.key()).count(),
                ITEM_COUNT * 2
            );
        }
    }

    for _ in 0..10 {
        let db = Database::builder(&folder).open()?;

        let keyspaces = &[
            db.keyspace("default1", KeyspaceCreateOptions::default)?,
            db.keyspace("default2", KeyspaceCreateOptions::default)?,
            db.keyspace("default3", KeyspaceCreateOptions::default)?,
        ];

        for tree in keyspaces {
            assert_eq!(tree.len()?, ITEM_COUNT * 2);
            assert_eq!(tree.iter().flat_map(|x| x.key()).count(), ITEM_COUNT * 2);
            assert_eq!(
                tree.iter().rev().flat_map(|x| x.key()).count(),
                ITEM_COUNT * 2
            );
        }
    }

    Ok(())
}
