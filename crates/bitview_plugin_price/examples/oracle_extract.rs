//! Each block's payments as the oracle filters them, as histogram bins, for tuning the oracle
//! offline: `<out>/offsets.u64` (one per height, plus the end) and `<out>/bins.u16`; and each
//! block's time, `<out>/timestamp.u32` (unix seconds, one per height).
//!
//! `cargo run --release --example oracle_extract --features litecoin -- <bitviewdir> <node dir> <out>`
//! with the daemon stopped (it holds the database's lock).

use std::{
    env,
    fs::File,
    io::{BufWriter, Write},
    path::PathBuf,
};

use bitview_plugin::ImportContext;
use bitview_plugin_indexer::Indexer;
use bitview_plugin_price::block_payments;
use brk_error::Result;
use brk_exit::Exit;
use brk_oracle::sats_to_bin;
use brk_reader::Reader;
use brk_rpc::{Auth, Client};
use vecdb::{AnyVec, Budgeted, ReadableVec};

const CHUNK: usize = 50_000;

fn main() -> Result<()> {
    let args: Vec<PathBuf> = env::args().skip(1).map(PathBuf::from).collect();
    let [data, node, out] = &args[..] else {
        panic!("usage: oracle_extract <bitviewdir> <node dir> <out>");
    };
    Budgeted::init_global(1 << 30)?;
    let client = Client::new(
        &format!(
            "http://127.0.0.1:{}",
            env::var("RPC_PORT").unwrap_or("9332".into())
        ),
        Auth::CookieFile(node.join(".cookie")),
    )?;
    let reader = Reader::new(node.join("blocks"), &client)?;
    let exit = Exit::new();
    let indexer = Indexer::import(ImportContext::new(data, &exit), &reader)?;
    let heights = indexer.vecs().transactions.first_tx_index.len();

    let mut offsets = BufWriter::new(File::create(out.join("offsets.u64"))?);
    let mut bins = BufWriter::new(File::create(out.join("bins.u16"))?);
    let mut timestamps = BufWriter::new(File::create(out.join("timestamp.u32"))?);
    let mut written = 0u64;
    for start in (0..heights).step_by(CHUNK) {
        let end = (start + CHUNK).min(heights);
        for timestamp in indexer.vecs().blocks.timestamp.collect_range_at(start, end) {
            timestamps.write_all(&timestamp.to_le_bytes())?;
        }
        let mut io = Ok(());
        block_payments(&indexer, start..end, None, |_, payments| {
            if io.is_err() {
                return;
            }
            io = (|| {
                offsets.write_all(&written.to_le_bytes())?;
                for bin in payments.into_iter().filter_map(sats_to_bin) {
                    bins.write_all(&(bin as u16).to_le_bytes())?;
                    written += 1;
                }
                std::io::Result::Ok(())
            })();
        })?;
        io?;
        eprintln!("{end}/{heights}: {written} payments");
    }
    offsets.write_all(&written.to_le_bytes())?;
    offsets.flush()?;
    bins.flush()?;
    timestamps.flush()?;
    Ok(())
}
