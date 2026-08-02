use lsm_tree::{AbstractTree, Config, Error, SequenceNumberCounter, get_tmp_folder};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, Write},
};

#[test]
fn v4_manifest_is_rejected_before_table_recovery() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();

    {
        let _tree = Config::new(
            folder.path(),
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;
    }

    let mut current = File::open(folder.path().join("current"))?;
    let mut version_bytes = [0; size_of::<u64>()];
    current.read_exact(&mut version_bytes)?;
    let manifest_path = folder
        .path()
        .join(format!("v{}", u64::from_le_bytes(version_bytes)));

    let reader = sfa::Reader::new(&manifest_path)?;
    let version_offset = reader
        .toc()
        .section(b"format_version")
        .expect("format version should exist")
        .pos();

    let mut manifest = OpenOptions::new().write(true).open(manifest_path)?;
    manifest.seek(std::io::SeekFrom::Start(version_offset))?;
    manifest.write_all(&[4])?;
    drop(manifest);

    assert!(matches!(
        Config::new(
            folder.path(),
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open(),
        Err(Error::InvalidVersion(4)),
    ));
    Ok(())
}

#[test]
fn v4_table_is_rejected_before_metadata_recovery() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();

    {
        let tree = Config::new(
            folder.path(),
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open()?;
        tree.insert("key", "value", 0);
        tree.flush_active_memtable(0)?;
    }

    let table_path = std::fs::read_dir(folder.path().join("tables"))?
        .next()
        .expect("table should exist")?
        .path();
    let reader = sfa::Reader::new(&table_path)?;
    let version_offset = reader
        .toc()
        .section(b"table_version")
        .expect("table version should exist")
        .pos();

    let mut table = OpenOptions::new().write(true).open(table_path)?;
    table.seek(std::io::SeekFrom::Start(version_offset))?;
    table.write_all(&[4])?;
    drop(table);

    assert!(matches!(
        Config::new(
            folder.path(),
            SequenceNumberCounter::default(),
            SequenceNumberCounter::default(),
        )
        .open(),
        Err(Error::InvalidVersion(4)),
    ));
    Ok(())
}
