use fjall::{Database, Error, FormatVersion, KeyspaceCreateOptions};

#[test]
fn new_database_uses_v4_and_reopens_fixed_width_tables() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;

    {
        let database = Database::builder(folder.path()).open()?;
        let keyspace = database.keyspace("fixed", KeyspaceCreateOptions::default)?;
        let mut ingestion = keyspace.start_ingestion()?;
        for index in 0..1_000_u64 {
            ingestion.write(index.to_be_bytes(), (index as u32).to_be_bytes())?;
        }
        ingestion.finish_exclusive()?;
    }

    assert_eq!(
        std::fs::read(folder.path().join("version"))?,
        [b'F', b'J', b'L', 4],
    );

    let database = Database::builder(folder.path()).open()?;
    let keyspace = database.keyspace("fixed", KeyspaceCreateOptions::default)?;
    assert_eq!(
        keyspace.get(777_u64.to_be_bytes())?.as_deref(),
        Some(777_u32.to_be_bytes().as_slice()),
    );
    Ok(())
}

#[test]
fn v3_database_requires_resync() -> fjall::Result<()> {
    let folder = tempfile::tempdir()?;
    std::fs::write(folder.path().join("version"), [b'F', b'J', b'L', 3])?;

    assert!(matches!(
        Database::builder(folder.path()).open(),
        Err(Error::InvalidVersion(Some(FormatVersion::V3))),
    ));
    Ok(())
}
