use fjall::{Database, KeyspaceCreateOptions};
use test_log::test;

fn pair(item: fjall::Result<fjall::KvPair>) -> fjall::Result<(Vec<u8>, Vec<u8>)> {
    let (key, value) = item?;
    Ok((key.to_vec(), value.to_vec()))
}

#[test]
fn standard_fast_paths_preserve_results_and_snapshot() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;
    let db = Database::builder(&folder).open()?;
    let tree = db.keyspace("standard", KeyspaceCreateOptions::default)?;

    let mut ingestion = tree.start_ingestion()?;
    ingestion.write("a1", "one")?;
    ingestion.write("a2", "two")?;
    ingestion.write("b1", "three")?;
    ingestion.write_weak_tombstone("c1")?;
    ingestion.finish_exclusive()?;

    assert_eq!(tree.get_standard("a2")?.as_deref(), Some(b"two".as_slice()));
    assert_eq!(tree.get_standard("c1")?, None);

    let all = tree
        .iter_standard()
        .map(pair)
        .collect::<fjall::Result<Vec<_>>>()?;
    assert_eq!(
        all,
        [
            (b"a1".to_vec(), b"one".to_vec()),
            (b"a2".to_vec(), b"two".to_vec()),
            (b"b1".to_vec(), b"three".to_vec()),
        ]
    );

    let prefix = tree
        .prefix_standard("a")
        .map(pair)
        .collect::<fjall::Result<Vec<_>>>()?;
    assert_eq!(prefix.len(), 2);

    let reverse_range = tree
        .range_standard("a2"..="b1")
        .rev()
        .map(pair)
        .collect::<fjall::Result<Vec<_>>>()?;
    assert_eq!(
        reverse_range,
        [
            (b"b1".to_vec(), b"three".to_vec()),
            (b"a2".to_vec(), b"two".to_vec()),
        ]
    );

    let snapshot_iter = tree.iter_standard();
    tree.insert("z1", "later")?;
    assert_eq!(snapshot_iter.count(), 3);

    Ok(())
}
