use fjall::{Database, KeyspaceCreateOptions};
use test_log::test;

#[test]
fn batch_simple() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    let db = Database::builder(&folder).open()?;
    let tree = db.keyspace("default", KeyspaceCreateOptions::default)?;
    let mut batch = db.batch();

    assert_eq!(tree.len()?, 0);
    batch.insert(&tree, "1", "abc");
    batch.insert(&tree, "3", "abc");
    batch.insert(&tree, "5", "abc");
    assert_eq!(tree.len()?, 0);

    batch.commit()?;
    assert_eq!(tree.len()?, 3);

    Ok(())
}

#[test]
fn batch_multi_keys() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    let db = Database::builder(&folder).open()?;
    let tree = db.keyspace("default", KeyspaceCreateOptions::default)?;
    let mut batch = db.batch();

    assert_eq!(tree.len()?, 0);
    batch.insert(&tree, "1", "abc");
    batch.insert(&tree, "1", "def");
    batch.insert(&tree, "1", "ghi");
    assert_eq!(tree.len()?, 0);

    batch.commit()?;
    assert_eq!(tree.len()?, 1);
    assert_eq!(&*tree.get("1")?.unwrap(), b"ghi");

    Ok(())
}
