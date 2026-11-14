---
name: merge-upstream
description: >-
  Merge or pull bitcoinresearchkit/brk (Bitview) into this Litecoin fork
  (lrk / Litview) without replaying the MWEB panic cascade. Use when the user
  says merge upstream, pull upstream, sync with BRK / Bitview, rebase on
  bitcoinresearchkit, or update from upstream main.
---

# Merge upstream Bitview into Litecoin

This repo is a Litecoin fork of [bitcoinresearchkit/brk](https://github.com/bitcoinresearchkit/brk),
now the Bitview plugin architecture (`bitview_plugin_*` crates composed in
`bitview_default`, run by `bitviewd`). Upstream encodes Bitcoin as a closed
sat ledger; Litecoin MWEB / HogEx breaks that. Everything Litecoin-specific
sits behind the `litecoin` Cargo feature, so the Bitcoin build must keep
compiling.

Read [reference.md](reference.md) for the MWEB facts, the port map and where
each accounting decision is enforced.

## When invoked

Copy this checklist and work it in order. Do not skip bindgen or the check script.

```
Merge progress:
- [ ] Fetch upstream, merge on a new branch
- [ ] Re-run scripts/litecoin/propagate_feature.py
- [ ] Both builds green (litecoin and bitcoin)
- [ ] Re-check every "enforced at" location in reference.md
- [ ] Bindgen WITH litecoin; MWEB series still in the client
- [ ] Studio presets build against the new client
- [ ] Run scripts/check.sh
- [ ] Smoke APIs before swapping the live binary
```

## 1. Fetch and merge

```bash
git remote add upstream https://github.com/bitcoinresearchkit/brk.git 2>/dev/null || true
git fetch upstream
git switch -c merge/upstream-$(date +%Y%m%d)
git merge upstream/main
```

Resolve conflicts keeping upstream's structure; re-apply Litecoin changes
behind `#[cfg(feature = "litecoin")]`. Prefer saturating math and the
`catch_unwind` airbag over upstream `panic!`.

**Never**

- `git push --force` to main
- Delete `/Volumes/LTC/brk` to "fix" the merge (the old live data)
- Run bindgen without `--features litecoin`
- Commit `docker/cloudflared/credentials.json` or `*.bak-*`

## 2. Feature graph and builds

New upstream crates need the `litecoin` feature forwarded:

```bash
python3 scripts/litecoin/propagate_feature.py
export CARGO_TARGET_DIR="$PWD/target"
cargo check -p bitviewd --features litecoin
cargo check --workspace
```

Re-run the script whenever a crate gains cfg code or a dependency.

## 3. Re-check the Litecoin seams

Upstream refactors move code: walk the "Where each decision is enforced" list
in reference.md and confirm each still applies. The usual breakages:

- a new `match` on `OutputType` without the `MWEBPegPool` / `MWEBPegIn` arms
  (storage dispatch must use `.storage_type()`);
- a new UTXO or distribution consumer reading raw outputs/inputs instead of
  the creations / spends journals (it must skip `is_mweb()`);
- a new per-transaction statistic that includes the HogEx;
- a new constant assuming 144 blocks/day, 210,000-block halvings or Bitcoin's
  RPC port (use `brk_types::TARGET_BLOCKS_PER_DAY`, `BLOCKS_PER_HALVING`,
  `brk_rpc::DEFAULT_RPC_PORT`);
- an upstream pool id colliding with `pools-ltc-v2.json` (Litecoin-only pools
  sit after upstream's last id; shift them and bump the file version).

Bump `bitview_plugin_indexer` `VERSION` (litecoin offset) if indexed data
changed: a full reindex into a fresh data dir is then expected.

## 4. Regenerate clients as Litecoin

```bash
rustup component add rustfmt   # bindgen formats the Rust client
CARGO_TARGET_DIR="$PWD/target" cargo bindgen --features litecoin
```

**Fail the merge** if `mweb`, `supply.totalSupply` or the `mwebPegPool` /
`mwebPegIn` output types disappeared from `modules/bitview-client/index.js`.
Do not hand-edit generated clients; regenerate.

Studio presets throw on a missing path. Build them against the new client
(and against a Bitcoin client, to keep the MWEB section guarded):

```bash
node -e 'import("./modules/bitview-client/index.js").then(async ({ BitviewClient }) => {
  const { studioPresets } = await import("./showcases/studio/presets.js");
  console.log(studioPresets(new BitviewClient("http://127.0.0.1:1"), { pools: new Set(), height: 3e6, year: 2026 }).length);
})'
```

## 5. Run the check script

```bash
.cursor/skills/merge-upstream/scripts/check.sh
```

Fix anything it prints, then re-run until exit 0.

## 6. Airbags (do not drop)

| Airbag | Where |
|---|---|
| `panic = "unwind"` | `[profile.release-ltc]` in the workspace `Cargo.toml` |
| `catch_unwind` around compute | `crates/bitview/src/update.rs` (litecoin cfg) |
| Saturating `Sats` `SubAssign` | `crates/brk_types/src/sats.rs` (litecoin cfg) |
| Address balance clamp | `crates/bitview_primitives/src/funded_addr_data.rs` (litecoin cfg) |
| Mining unclaimed-reward clamp | `crates/bitview_plugin_mining/src/rewards/compute.rs` |
| Guarded MWEB presets (the old `safeSection`) | `showcases/studio/presets.js` |

## 7. Smoke before live swap

Build and run against a separate data dir; never point a new build at
`/Volumes/LTC/brk`:

```bash
CARGO_TARGET_DIR="$PWD/target" cargo build --profile release-ltc -p bitviewd --features litecoin
curl -sf http://127.0.0.1:7070/health
curl -sf 'http://127.0.0.1:7070/api/series/search?q=mweb' | head -c 200
curl -sf http://127.0.0.1:7070/api/series/mweb_supply/height/len
curl -sf http://127.0.0.1:7070/api/series/supply_invariant_mismatch/height | tail -c 200
```

`mweb_accounting_mismatch`, `mweb_link_mismatch` and
`supply_invariant_mismatch` must be 0 at every height.

Live deploy only when asked: `docker/services/*.sh` and `install.sh` run the
live service, so update them to `bitviewd` / `release-ltc` as part of that swap.

## Conflict rules

- **Sats / balances / cost basis / fees / rewards:** keep saturating / `checked_sub`. Upstream `panic!` loses.
- **MWEB outputs:** never in the transparent supply, cohorts or address state; never counted as burned.
- **HogEx:** its fee stays in block fees; out of tx count, volume, fee-rate / vsize distributions and relay policy.
- **Price:** Litecoin has no on-chain oracle; `bitview_plugin_price` reads exchange closes under litecoin.
- **VERSION bumps:** say so; compute will replay. Do not wipe a data dir unless the user asks.

## Afterward

Summarize: what upstream brought, which Litecoin seams needed re-porting,
bindgen result, check.sh output, and whether the live swap is still pending.
