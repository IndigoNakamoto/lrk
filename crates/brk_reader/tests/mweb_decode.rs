//! Decodes real Litecoin blocks around and after MWEB activation and checks the
//! decoded extension block against litecoind's own accounting (`getblockheader`
//! `mweb_amount`, `getblockstats`). Fixtures come from
//! `scripts/litecoin/mweb_fixtures.py`; set `MWEB_FIXTURES_DIR` to run the same
//! checks over a larger sample.

use std::{fs, path::PathBuf};

use litecoin::{Block, WitnessVersion, consensus::encode::deserialize_hex};
use serde_json::Value;

const MWEB_ACTIVATION_HEIGHT: u64 = 2_265_984;

fn fixture_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")];
    if let Ok(extra) = std::env::var("MWEB_FIXTURES_DIR") {
        dirs.push(PathBuf::from(extra));
    }
    dirs
}

fn sat(v: &Value, key: &str) -> i64 {
    v[key]
        .as_i64()
        .unwrap_or_else(|| panic!("missing {key} in fixture"))
}

fn check(height: u64, block: &Block, r: &Value) {
    assert_eq!(block.txdata.len() as i64, sat(r, "tx_count"), "{height}: tx count");
    assert_eq!(
        sat(r, "coinbase_value"),
        sat(r, "subsidy") + sat(r, "total_fee"),
        "{height}: coinbase == subsidy + fees"
    );

    let hogex_positions: Vec<usize> = block
        .txdata
        .iter()
        .enumerate()
        .filter(|(_, tx)| tx.is_hog_ex)
        .map(|(i, _)| i)
        .collect();

    if height < MWEB_ACTIVATION_HEIGHT {
        assert!(block.mweb_block.is_none(), "{height}: MWEB before activation");
        assert!(hogex_positions.is_empty(), "{height}: HogEx before activation");
        return;
    }

    assert_eq!(hogex_positions, vec![block.txdata.len() - 1], "{height}: HogEx must be last and unique");
    let mweb = block
        .mweb_block
        .as_ref()
        .unwrap_or_else(|| panic!("{height}: missing extension block"));
    let hogex = block.txdata.last().unwrap();

    // HogAddr is HogEx vout[0], a witness v8 output holding the whole MWEB supply.
    let hogaddr = &hogex.output[0];
    assert_eq!(hogaddr.script_pubkey.witness_version(), Some(WitnessVersion::V8), "{height}: HogAddr is v8");
    assert_eq!(hogaddr.value.to_sat() as i64, sat(r, "mweb_amount"), "{height}: HogAddr == mweb_amount");

    if let Some(prev_txid) = r["prev_hogex_txid"].as_str() {
        let first = &hogex.input[0].previous_output;
        assert_eq!(first.txid.to_string(), prev_txid, "{height}: HogEx input 0 spends previous HogAddr");
        assert_eq!(first.vout, 0, "{height}: previous HogAddr is vout 0");
    } else {
        let in_block: Vec<_> = block.txdata.iter().map(|tx| tx.compute_txid()).collect();
        assert!(
            hogex.input.iter().all(|i| in_block.contains(&i.previous_output.txid)),
            "{height}: activation HogEx inputs are in-block peg-ins"
        );
    }

    let kernels = &mweb.tx_body.kernels;
    let pegin_kernels: i64 = kernels.iter().filter_map(|k| k.pegin).sum();
    let pegout_kernels: i64 = kernels.iter().flat_map(|k| &k.pegouts).map(|p| p.amount).sum();
    let fees: i64 = kernels.iter().filter_map(|k| k.fee).sum();

    let pegin_outputs: i64 = block.txdata[..block.txdata.len() - 1]
        .iter()
        .flat_map(|tx| &tx.output)
        .filter(|o| o.script_pubkey.witness_version() == Some(WitnessVersion::V9))
        .map(|o| o.value.to_sat() as i64)
        .sum();
    let pegout_outputs: i64 = hogex.output[1..].iter().map(|o| o.value.to_sat() as i64).sum();

    assert_eq!(pegin_kernels, pegin_outputs, "{height}: kernel peg-ins == v9 outputs");
    assert_eq!(pegout_kernels, pegout_outputs, "{height}: kernel peg-outs == HogEx vout[1..]");
    assert_eq!(fees, sat(r, "hogex_fee"), "{height}: kernel fees == HogEx in - out");
    assert_eq!(
        sat(r, "prev_mweb_amount") + pegin_kernels - pegout_kernels - fees,
        sat(r, "mweb_amount"),
        "{height}: MWEB supply accounting"
    );

    assert_eq!(mweb.header.height as u64, height, "{height}: MWEB header height");
    assert_eq!(mweb.header.output_mmr_size as i64, sat(r, "num_txos"), "{height}: output MMR size");
    let num_kernels = sat(r, "num_kernels");
    assert!(
        kernels.len() as i64 == num_kernels || mweb.header.kernel_mmr_size as i64 == num_kernels,
        "{height}: num_kernels {num_kernels} matches neither kernels {} nor kernel MMR {}",
        kernels.len(),
        mweb.header.kernel_mmr_size
    );
}

#[test]
fn decodes_mweb_blocks_against_node_accounting() {
    let mut checked = 0;
    for dir in fixture_dirs() {
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        let mut heights: Vec<u64> = entries
            .filter_map(|e| e.ok()?.path().file_stem()?.to_str()?.parse().ok())
            .collect();
        heights.sort_unstable();
        heights.dedup();
        for height in heights {
            let hex = fs::read_to_string(dir.join(format!("{height}.hex"))).unwrap();
            let r: Value = serde_json::from_str(&fs::read_to_string(dir.join(format!("{height}.json"))).unwrap()).unwrap();
            let block: Block = deserialize_hex(hex.trim()).unwrap_or_else(|e| panic!("{height}: decode failed: {e}"));
            check(height, &block, &r);
            checked += 1;
        }
    }
    assert!(checked >= 3, "expected at least the bundled fixtures, checked {checked}");
    eprintln!("checked {checked} blocks");
}
