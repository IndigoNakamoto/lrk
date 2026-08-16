// Regression test for https://github.com/fjall-rs/fjall/issues/68

use fjall::{Database, KeyspaceCreateOptions};

#[test_log::test]
fn journal_recover_large_value() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    let large_value = "a".repeat(128_000);

    {
        let db = Database::builder(&folder).open()?;
        let tree = db.keyspace("default", KeyspaceCreateOptions::default)?;
        tree.insert("a", &large_value)?;
        tree.insert("b", "b")?;
    }

    {
        let db = Database::builder(&folder).open()?;
        let tree = db.keyspace("default", KeyspaceCreateOptions::default)?;
        assert_eq!(large_value.as_bytes(), &*tree.get("a")?.unwrap());
        assert_eq!(b"b", &*tree.get("b")?.unwrap());
    }

    Ok(())
}
