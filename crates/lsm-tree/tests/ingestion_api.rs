use lsm_tree::{AbstractTree, Config, SeqNo, SequenceNumberCounter, get_tmp_folder};

#[test]
fn tree_ingestion_tombstones_delete_existing_keys() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();

    let tree = Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    for i in 0..10u32 {
        let key = format!("k{:03}", i);
        tree.insert(key.as_bytes(), b"v", 0);
    }

    let mut ingest = tree.ingestion()?;
    for i in 0..10u32 {
        let key = format!("k{:03}", i);
        ingest.write_tombstone(key)?;
    }
    ingest.finish()?;

    for i in 0..10u32 {
        let key = format!("k{:03}", i);
        assert!(tree.get(key.as_bytes(), SeqNo::MAX)?.is_none());
    }
    assert_eq!(tree.tombstone_count(), 10);

    Ok(())
}

#[test]
fn sealed_memtable_value_overrides_table_value() -> lsm_tree::Result<()> {
    use lsm_tree::AbstractTree;
    let folder = get_tmp_folder();

    let tree = lsm_tree::Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    // Older table value via ingestion (seqno 1)
    {
        let mut ingest = tree.ingestion()?;
        ingest.write(b"k", b"old")?;
        ingest.finish()?;
    }

    // Newer value in memtable (seqno 2), then seal it
    tree.insert(b"k", b"new", 2);
    let _ = tree.rotate_memtable(); // move active -> sealed

    // Read should return the sealed memtable value
    assert_eq!(
        tree.get(b"k", lsm_tree::SeqNo::MAX)?,
        Some(b"new".as_slice().into())
    );

    Ok(())
}

#[test]
fn sealed_memtable_tombstone_overrides_table_value() -> lsm_tree::Result<()> {
    use lsm_tree::AbstractTree;
    let folder = get_tmp_folder();

    let tree = lsm_tree::Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    // Older table value via ingestion (seqno 1)
    {
        let mut ingest = tree.ingestion()?;
        ingest.write(b"k", b"old")?;
        ingest.finish()?;
    }

    // Newer tombstone in memtable (seqno 2), then seal it
    tree.remove(b"k", 2);
    let _ = tree.rotate_memtable();

    // Read should see the delete from sealed memtable
    assert!(tree.get(b"k", lsm_tree::SeqNo::MAX)?.is_none());

    Ok(())
}

#[test]
fn tables_newest_first_returns_highest_seqno() -> lsm_tree::Result<()> {
    use lsm_tree::AbstractTree;
    let folder = get_tmp_folder();

    let tree = lsm_tree::Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    // Two separate ingestions create two tables containing the same key at different seqnos
    {
        let mut ingest = tree.ingestion()?;
        ingest.write(b"k", b"v1")?;
        ingest.finish()?;
    }
    {
        let mut ingest = tree.ingestion()?;
        ingest.write(b"k", b"v2")?;
        ingest.finish()?;
    }

    // With memtables empty, read should return the value from the newest table run (seqno 2)
    assert_eq!(
        tree.get(b"k", lsm_tree::SeqNo::MAX)?,
        Some(b"v2".as_slice().into())
    );
    Ok(())
}

#[test]
#[should_panic(expected = "next key in ingestion must be greater than last key")]
fn ingestion_enforces_order_standard_panics() {
    let folder = tempfile::tempdir().unwrap();
    let tree = lsm_tree::Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()
    .unwrap();

    let mut ingest = tree.ingestion().unwrap();

    // First write higher key, then lower to trigger ordering assertion
    ingest.write(b"k2", b"v").unwrap();

    // Panics here
    let _ = ingest.write(b"k1", b"v");
}
#[test]
fn memtable_put_overrides_table_tombstone() -> lsm_tree::Result<()> {
    use lsm_tree::AbstractTree;
    let folder = get_tmp_folder();

    let tree = lsm_tree::Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    // Older put written via ingestion to tables (seqno 1)
    {
        let mut ingest = tree.ingestion()?;
        ingest.write(b"k", b"v1")?;
        ingest.finish()?;
    }

    // Newer tombstone written via ingestion to tables (seqno 2)
    {
        let mut ingest = tree.ingestion()?;
        ingest.write_tombstone(b"k")?;
        ingest.finish()?;
    }

    // Newest put in active memtable (seqno 3) should override table tombstone
    tree.insert(b"k", b"v3", 3);
    assert_eq!(
        tree.get(b"k", lsm_tree::SeqNo::MAX)?,
        Some(b"v3".as_slice().into())
    );
    Ok(())
}
#[test]
fn tree_ingestion_finish_no_writes_noop() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();

    let tree = Config::new(
        &folder,
        SequenceNumberCounter::default(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    let before_tables = tree.table_count();
    tree.ingestion()?.finish()?;
    let after_tables = tree.table_count();

    assert_eq!(before_tables, after_tables);
    assert!(tree.is_empty(SeqNo::MAX, None)?);

    Ok(())
}
