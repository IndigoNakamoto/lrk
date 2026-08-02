use lsm_tree::{AbstractTree, Config, SequenceNumberCounter, get_tmp_folder};
use test_log::test;

#[test]
fn tree_recovery_cleanup_orphans() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();

    {
        let tree = Config::new(
            &folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;
        tree.insert("a", "a", 0);
        tree.flush_active_memtable(0)?;

        assert!(folder.path().join("tables").join("0").try_exists()?);

        tree.major_compact(u64::MAX, 0)?;

        assert!(folder.path().join("tables").join("1").try_exists()?);
    }

    std::fs::File::create(folder.path().join("tables").join("0"))?;

    {
        let _tree = Config::new(
            &folder,
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;

        assert!(!folder.path().join("tables").join("0").try_exists()?);
        assert!(folder.path().join("tables").join("1").try_exists()?);
    }

    Ok(())
}
