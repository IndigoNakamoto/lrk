use lsm_tree::{AbstractTree, Config, SequenceNumberCounter, get_tmp_folder};
use test_log::test;

#[test]
fn exclusive_point_read_tracks_latest_version() -> lsm_tree::Result<()> {
    let folder = get_tmp_folder();
    let seqno = SequenceNumberCounter::default();
    let tree = Config::new(
        folder.path(),
        seqno.clone(),
        SequenceNumberCounter::default(),
    )
    .open()?;

    tree.insert("a", "first", seqno.next());
    tree.flush_active_memtable(0)?;
    assert_eq!(
        tree.get_exclusive("a")?.as_deref(),
        Some(b"first".as_slice())
    );

    tree.insert("b", "second", seqno.next());
    tree.flush_active_memtable(0)?;
    assert_eq!(
        tree.get_exclusive("a")?.as_deref(),
        Some(b"first".as_slice())
    );
    assert_eq!(
        tree.get_exclusive("b")?.as_deref(),
        Some(b"second".as_slice())
    );

    tree.major_compact(u64::MAX, 1_000)?;
    assert_eq!(
        tree.get_exclusive("a")?.as_deref(),
        Some(b"first".as_slice())
    );
    assert_eq!(
        tree.get_exclusive("b")?.as_deref(),
        Some(b"second".as_slice())
    );

    Ok(())
}
