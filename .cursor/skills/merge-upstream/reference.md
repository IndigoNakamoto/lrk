# Litecoin delta vs upstream Bitview

Upstream (bitcoinresearchkit/brk, v0.13+) is the Bitview plugin
architecture: `bitview_plugin_*` crates composed in `bitview_default`, run by
`bitviewd`. The last pre-Bitview Litecoin fork is tagged `lrk-v0.11.2-final`.

Why merges hurt: upstream assumes every sat is accounted for (`received >= sent`,
coinbase >= fees). MWEB pegs, HogEx, and unresolved prevouts wrap `u64` and
used to abort the process.

## MWEB facts (verified against litecoind 0.21.5.5)

- Activation height 2,265,984 (`softforks.mweb.bip8.since`).
- HogEx is the last tx of every post-activation block (`Transaction::is_hog_ex`).
  `vout[0]` is HogAddr (witness v8) and equals `getblockheader.mweb_amount`.
  `vout[1..]` are peg-outs. Input 0 spends the previous HogAddr; the rest are
  peg-ins (witness v9 outputs created earlier in the same block).
- HogEx input - output == sum of MWEB kernel fees; the coinbase claims it
  (`coinbase == subsidy + getblockstats.totalfee`).
- `mweb_header.num_txos` is the cumulative output MMR size, not a block count.
- Decoder: `litecoin` crate `Block` / `MwebBlock`; proven by
  `crates/brk_reader/tests/mweb_decode.rs` (fixtures from
  `scripts/litecoin/mweb_fixtures.py`, `MWEB_FIXTURES_DIR` for big samples).

## Port map (fork v0.11.2 -> Bitview)

| Fork file | Bitview home |
|---|---|
| `brk_chain` (primitives = bitcoin / litecoin, `ChainConstants`) | `crates/brk_chain` (reintroduced, `litecoin` feature) |
| `brk_reader/src/scan.rs` magic `0xFBC0B6DB` | `crates/brk_reader/src` (litecoin cfg) |
| `brk_reader/src/parse.rs` `mweb_block: None` | `crates/brk_reader/src/parse.rs`: decode extension block, non-fatal on failure |
| `brk_types/src/block.rs` MWEB txid guard (flag `0x08`) | `crates/brk_types/src/block.rs` |
| `brk_types/src/output_type.rs` `MWEBPegPool` / `MWEBPegIn` | same path |
| `brk_types/src/sats.rs` saturating `SubAssign` | same path, litecoin cfg |
| `brk_types/src/funded_addr_data.rs` checked balance | `crates/bitview_primitives/src/funded_addr_data.rs`, litecoin cfg |
| `brk_rpc/src/methods.rs` local `BlockchainInfo` (ignores `bip8`) | `crates/brk_rpc/src/rpc_client/{methods,responses}.rs` |
| `brk_indexer/src/processor/txout/mod.rs` MWEB as unknown script | `crates/bitview_plugin_indexer/src/processor/txout/mod.rs` |
| `brk_indexer` `is_hog_ex` / `has_mw_tx` (never written on the fork) | `bitview_plugin_indexer` tx feature flags `HOGEX` / `MWEB_TX` |
| `brk_computer/src/outputs/mweb/*`, `transactions/hogex/*` | `crates/bitview_plugin_mweb` |
| `brk_computer/src/mining/rewards/compute.rs` clamps | `crates/bitview_plugin_mining`, litecoin cfg |
| `brk_computer/src/distribution/*` underflow clamps | `crates/bitview_distribution` / `bitview_plugin_*` cohorts, litecoin cfg |
| `brk_computer/src/blocks/mod.rs` 576 blocks/day, difficulty / 4096 | `bitview_plugin_blocks`, `bitview_plugin_indexer/src/constants.rs`, `blk/src/fields.rs` |
| `brk_computer/src/pools` + `brk_types/pools-ltc-v1.json` | `crates/bitview_primitives/src/pools.rs` |
| `brk_cohort` MWEB unspendable buckets | `crates/bitview_cohort/src/by_type.rs` |
| `brk_cli/src/main.rs` `catch_unwind` | `bitviewd` run loop, litecoin cfg |
| `panic = "unwind"` | `[profile.release-ltc]` (Cargo can't feature-gate profiles) |

## Accounting decisions (keep)

- Distribution excludes v8/v9 outputs and HogEx spends of them, so realized cap,
  MVRV, cointime, coinflow and cohorts are transparent-only (tagged in series
  metadata). Otherwise ~544k LTC of HogAddr gets re-priced at spot every block.
- `circulating_supply` is therefore transparent; `total_supply = circulating +
  mweb_supply`; `supply_invariant_mismatch` must stay 0.
- Ordinary tx count / volume / size / fee-rate skip HogEx. Its fee stays in
  block fees (it is real MWEB fee revenue).

Where each decision is enforced (re-check these on every upstream merge):

- UTXO history journals: `bitview_plugin_outputs/src/value/compute.rs`
  (creations skip `is_mweb()`, never counted as burned) and
  `bitview_plugin_inputs/src/spends.rs` (spends of them skipped). Age, holders,
  cointime, coinflow, profitability, entry and the UTXO set all read these.
- Per-block UTXO cohorts: `bitview_plugin_utxos/src/state/received.rs` and
  `block/detailed_spends.rs` (`SpendableType::get_mut` panics on MWEB types).
- Addresses: `OutputType::is_addr` is false for MWEB.
- Per-type output/input counts: `OutputTypeId::MwebPegPool` / `MwebPegIn` and
  `ByType::mweb` in `bitview_cohort` (series `mweb_peg_pool_*`, `mweb_peg_in_*`).
- Transactions: `bitview_plugin_transactions/src/transparent.rs` (tx count),
  `fees/compute/block.rs` (`Block::hogex`: volume, distributions, CPFP
  clusters). Relay policy exempts HogEx in the indexer's
  `processor/transaction/analysis/mod.rs`.
- Supply: `bitview_plugin_supply/src/mweb.rs` (`total_supply`,
  `supply_invariant_mismatch = total - (scheduled subsidy - burned)`). Under
  `litecoin`, supply computes after holders in `bitview_default/src/compute.rs`.

## Runtime (do not invent new paths)

- Old live data: `BRK_DATA_DIR=/Volumes/LTC/brk` (never wipe). Bitview data
  goes to a separate dir; a full reindex is required.
- Litecoind default datadir `~/Library/Application Support/Litecoin`, txindex on.
- Launchd: `com.litview.brk`, `com.litview.watchdog`, `com.litview.cloudflared`
- Public: `https://litview.space` -> `127.0.0.1:7070`
- Never commit `docker/cloudflared/*.bak-*` or `credentials.json`
