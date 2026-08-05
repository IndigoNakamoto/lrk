use std::{collections::HashSet, env, path::Path, time::Instant};

use brk_indexer::Indexer;
use brk_types::{Height, OutputType, TypeIndex};
use vecdb::{AnyVec, ReadableVec};

#[derive(Default)]
struct Counts {
    events: u64,
    new: u64,
    cached: u64,
    disk_reads: u64,
    unique_disk_reads: u64,
}

fn first_index(indexer: &Indexer, output_type: OutputType, height: Height) -> TypeIndex {
    match output_type {
        OutputType::P2PK65 => indexer.vecs.addrs.p2pk65.first_index.collect_one(height).into(),
        OutputType::P2PK33 => indexer.vecs.addrs.p2pk33.first_index.collect_one(height).into(),
        OutputType::P2PKH => indexer.vecs.addrs.p2pkh.first_index.collect_one(height).into(),
        OutputType::P2SH => indexer.vecs.addrs.p2sh.first_index.collect_one(height).into(),
        OutputType::P2WPKH => indexer.vecs.addrs.p2wpkh.first_index.collect_one(height).into(),
        OutputType::P2WSH => indexer.vecs.addrs.p2wsh.first_index.collect_one(height).into(),
        OutputType::P2TR => indexer.vecs.addrs.p2tr.first_index.collect_one(height).into(),
        OutputType::P2A => indexer.vecs.addrs.p2a.first_index.collect_one(height).into(),
        _ => unreachable!(),
    }
}

fn main() {
    let outputs_dir = Path::new(&env::var("HOME").unwrap()).join(".brk");
    let indexer = Indexer::forced_import(&outputs_dir).unwrap();

    let from = 950_001usize;
    let to = 960_001usize;
    let start = Instant::now();

    let output_starts = indexer
        .vecs
        .outputs
        .first_txout_index
        .collect_range_at(from, to + 1);
    let input_starts = indexer
        .vecs
        .inputs
        .first_txin_index
        .collect_range_at(from, to + 1);

    let output_from = output_starts[0].to_usize();
    let output_types = indexer
        .vecs
        .outputs
        .output_type
        .collect_range_at(output_from, output_starts.last().unwrap().to_usize());
    let output_indexes = indexer
        .vecs
        .outputs
        .type_index
        .collect_range_at(output_from, output_starts.last().unwrap().to_usize());

    let input_from = input_starts[0].to_usize();
    let input_types = indexer
        .vecs
        .inputs
        .output_type
        .collect_range_at(input_from, input_starts.last().unwrap().to_usize());
    let input_indexes = indexer
        .vecs
        .inputs
        .type_index
        .collect_range_at(input_from, input_starts.last().unwrap().to_usize());

    let mut cache = HashSet::new();
    let mut counts = Counts::default();

    for block_offset in 0..to - from {
        let height = Height::from(from + block_offset);
        let first_indexes = OutputType::ADDR_TYPES.map(|output_type| {
            (
                output_type,
                first_index(&indexer, output_type, height),
            )
        });
        let mut block_misses = HashSet::new();
        let mut block_addresses = HashSet::new();

        let output_start = output_starts[block_offset].to_usize() - output_from;
        let output_end = output_starts[block_offset + 1].to_usize() - output_from;
        let output_items = output_types[output_start..output_end]
            .iter()
            .copied()
            .zip(output_indexes[output_start..output_end].iter().copied());

        let input_start = input_starts[block_offset].to_usize() - input_from + 1;
        let input_end = input_starts[block_offset + 1].to_usize() - input_from;
        let input_items = input_types[input_start..input_end]
            .iter()
            .copied()
            .zip(input_indexes[input_start..input_end].iter().copied());

        for (output_type, type_index) in output_items.chain(input_items) {
            if !output_type.is_addr() {
                continue;
            }
            counts.events += 1;
            let key = (output_type, type_index);
            block_addresses.insert(key);

            let first = first_indexes
                .iter()
                .find_map(|(kind, first)| (*kind == output_type).then_some(*first))
                .unwrap();
            if type_index >= first {
                counts.new += 1;
            } else if cache.contains(&key) {
                counts.cached += 1;
            } else {
                counts.disk_reads += 1;
                block_misses.insert(key);
            }
        }

        counts.unique_disk_reads += block_misses.len() as u64;
        cache.extend(block_addresses);
    }

    println!("elapsed={:?}", start.elapsed());
    println!("events={}", counts.events);
    println!("new={}", counts.new);
    println!("cached={}", counts.cached);
    println!("disk_reads={}", counts.disk_reads);
    println!("unique_disk_reads={}", counts.unique_disk_reads);
    println!(
        "redundant_disk_reads={} ({:.2}%)",
        counts.disk_reads - counts.unique_disk_reads,
        100.0 * (counts.disk_reads - counts.unique_disk_reads) as f64 / counts.disk_reads as f64
    );
}
