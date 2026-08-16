use fjall::{Database, Readable, Slice};
use test_log::test;

#[test]
fn ingest_recovery() -> fjall::Result<()> {
    let path = tempfile::tempdir()?;

    let ks = "default";
    let key: Slice = b"abc".into();
    let value: Slice = b"zzz".into();

    {
        let db = Database::builder(&path).open()?;
        let keyspace = db.keyspace(ks, Default::default)?;
        let mut ing = keyspace.start_ingestion()?;
        ing.write(key.clone(), value.clone())?;
        ing.finish()?;
        assert_eq!(keyspace.get(key.clone())?, Some(value.clone())); // ok
    }

    {
        let db = Database::builder(&path).open()?;
        let keyspace = db.keyspace(ks, Default::default)?;
        assert_eq!(keyspace.get(key.clone())?, Some(value.clone())); // ok
    }

    {
        let db = Database::builder(&path).open()?;
        let keyspace = db.keyspace(ks, Default::default)?;
        let snapshot = db.snapshot();
        assert_eq!(snapshot.get(&keyspace, key.clone())?, Some(value.clone())); // snapshot - not ok
    }

    Ok(())
}

#[test]
fn concurrent_exclusive_ingest_recovery() -> fjall::Result<()> {
    let path = tempfile::tempdir()?;

    {
        let db = Database::builder(&path).open()?;
        let first = db.keyspace("first", Default::default)?;
        let second = db.keyspace("second", Default::default)?;

        let (first_result, second_result) = std::thread::scope(|scope| {
            let first = scope.spawn(|| {
                let mut ingestion = first.start_ingestion()?;
                ingestion.write(b"a", b"first")?;
                ingestion.finish_exclusive()
            });
            let second = scope.spawn(|| {
                let mut ingestion = second.start_ingestion()?;
                ingestion.write(b"b", b"second")?;
                ingestion.finish_exclusive()
            });

            (first.join().unwrap(), second.join().unwrap())
        });
        first_result?;
        second_result?;
    }

    {
        let db = Database::builder(&path).open()?;
        let first = db.keyspace("first", Default::default)?;
        let second = db.keyspace("second", Default::default)?;
        assert_eq!(first.get(b"a")?.as_deref(), Some(b"first".as_slice()));
        assert_eq!(second.get(b"b")?.as_deref(), Some(b"second".as_slice()));
    }

    Ok(())
}
