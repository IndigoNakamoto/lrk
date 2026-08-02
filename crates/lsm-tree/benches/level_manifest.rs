use criterion::{Criterion, criterion_group, criterion_main};
use lsm_tree::{AbstractTree, Config, config::BlockSizePolicy};

fn iterate_tables(c: &mut Criterion) {
    let mut group = c.benchmark_group("Iterate version tables");
    group.sample_size(10);

    std::fs::create_dir_all(".bench").unwrap();

    for segment_count in [0, 1, 5, 10, 100, 500, 1_000, 2_000, 4_000] {
        group.bench_function(format!("iterate {segment_count} segments"), |b| {
            let folder = tempfile::tempdir_in(".bench").unwrap();
            let tree = Config::new(folder, Default::default(), Default::default())
                .data_block_size_policy(BlockSizePolicy::all(1_024))
                .open()
                .unwrap();

            for x in 0_u64..segment_count {
                tree.insert("a", "b", x);
                tree.flush_active_memtable(0).unwrap();
            }

            let version = tree.current_version();

            b.iter(|| {
                assert_eq!(version.iter_tables().count(), segment_count as usize);
            });
        });
    }
}

criterion_group!(benches, iterate_tables);
criterion_main!(benches);
