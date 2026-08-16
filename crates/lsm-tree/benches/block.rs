use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use lsm_tree::{
    CompressionType,
    table::block::{Block, BlockType},
};

fn input(size: usize) -> Vec<u8> {
    (0..size)
        .map(|index| {
            let index = index as u64;
            index
                .wrapping_mul(6_364_136_223_846_793_005)
                .rotate_left(17) as u8
        })
        .collect()
}

fn encode_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encode block");

    for compression in [CompressionType::None, CompressionType::Lz4] {
        for kib in [4, 8, 16, 32, 64, 128] {
            let data = input(kib * 1_024);
            group.throughput(Throughput::Bytes(data.len() as u64));

            group.bench_function(format!("{kib} KiB [{compression}]"), |b| {
                b.iter_batched(
                    || Vec::with_capacity(data.len()),
                    |mut encoded| {
                        Block::write_into(&mut encoded, &data, BlockType::Data, compression)
                            .unwrap();
                        encoded
                    },
                    BatchSize::SmallInput,
                );
            });
        }
    }
}

fn decode_block(c: &mut Criterion) {
    let mut group = c.benchmark_group("Decode block");

    for compression in [CompressionType::None, CompressionType::Lz4] {
        for kib in [4, 8, 16, 32, 64, 128] {
            let data = input(kib * 1_024);
            let mut encoded = Vec::with_capacity(data.len());
            Block::write_into(&mut encoded, &data, BlockType::Data, compression).unwrap();

            group.throughput(Throughput::Bytes(data.len() as u64));
            group.bench_function(format!("{kib} KiB [{compression}]"), |b| {
                b.iter(|| {
                    let block = Block::from_reader(&mut &encoded[..], compression).unwrap();
                    assert_eq!(block.data.len(), data.len());
                });
            });
        }
    }
}

criterion_group!(benches, encode_block, decode_block);
criterion_main!(benches);
