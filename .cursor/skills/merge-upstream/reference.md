# Litecoin delta vs upstream BRK

Why merges hurt: BRK assumes every sat is accounted for (`received ≥ sent`,
coinbase ≥ fees). MWEB pegs, HogEx, and unresolved prevouts (`Sats::MAX`)
wrap `u64` and used to abort the process.

## Panic surface — diff these first

```
crates/brk_types/src/sats.rs
crates/brk_types/src/funded_addr_data.rs
crates/brk_types/src/empty_addr_data.rs
crates/brk_types/src/supply_state.rs
crates/brk_computer/src/distribution/
crates/brk_computer/src/mining/rewards/
crates/brk_computer/src/transactions/fees/
crates/brk_computer/src/investing/import.rs
crates/brk_indexer/src/processor/txout/mod.rs
crates/brk_rpc/src/methods.rs
crates/brk_query/src/impl/block/info.rs
crates/brk_cli/src/main.rs
Cargo.toml
modules/brk-client/index.js
website/scripts/options/network.js
website/scripts/options/partial.js
```

New `panic!`, `unreachable!`, or `checked_sub(...).unwrap()` in that set is
the next crash.

## Known Litecoin patches (keep)

| Issue | Patch |
|---|---|
| MWEB outputs `unreachable!` | Indexer treats `MWEBPegPool` / `MWEBPegIn` like unknown scripts |
| `softforks.*.type = "bip8"` | Local `BlockchainInfo` in `brk_rpc` (ignore extra fields) |
| First USD day 2010-07-12 | Litecoin `Date::new(2013, 4, 1)` in investing import |
| `Sats::MAX` prevouts → fake fees | `tx_fee` saturates; fee VERSION 4 |
| `coinbase < fees` | Subsidy clamps to 0 |
| Wrapping `received - sent` | `FundedAddrData::balance` checked + 84M LTC cap |
| Empty-wallet convert panic | `max(sent, received)` |
| Cohort / cost-basis underflow | Early-return or saturate to 0 |
| `Sats::SubAssign` panic | Saturate to 0 |
| Explorer OOB while extras empty | Clamp extras len + indexer-only fallback |
| JS client missing MWEB | Bindgen `--features bindgen,litecoin` + `outputs.mweb` |
| Browse tree `TypeError` | Guard `outputs.mweb`; `safeSection` per folder |
| Process death | `panic = "unwind"` + `catch_unwind` around compute |

## Runtime (do not invent new paths)

- Data: `BRK_DATA_DIR=/Volumes/LTC/brk`, chain `/Volumes/LTC/litecoin`
- Launchd: `com.litview.brk`, `com.litview.watchdog`, `com.litview.cloudflared`
- Logs: `~/Library/Logs/litview/brk.out.log`
- Public: `https://litview.space` → `127.0.0.1:7070`
- Binary: `~/Library/Application Support/litview/bin/brk`
- Never commit `docker/cloudflared/*.bak-*` or `credentials.json`

Red `getblocktemplate` / `{"rules": ["mweb", "segwit"]}` is mempool noise, not compute.

## Cost-basis honesty

Saturating underflows keeps compute moving. Realized price / MVRV may stay
empty or wrong after a dirty pass. A VERSION bump that rebuilds those vecs
needs a planned replay, not more clamps. Do not wipe the SSD unless asked.

## Bindgen output

```
crates/brk_client/src/lib.rs
modules/brk-client/index.js          # hardlinked as website/scripts/modules/brk-client/index.js
packages/brk_client/brk_client/__init__.py
website/ + website_next/ LLM stubs
```
