---
name: merge-upstream
description: >-
  Merge or pull bitcoinresearchkit/brk into this Litecoin fork (lrk / Litview)
  without replaying the MWEB panic cascade. Use when the user says merge
  upstream, pull upstream, sync with BRK, rebase on bitcoinresearchkit, or
  update from v0.11.x / next.
---

# Merge upstream BRK into Litecoin

This repo is a Litecoin fork of [bitcoinresearchkit/brk](https://github.com/bitcoinresearchkit/brk).
Upstream encodes Bitcoin as a closed sat ledger. Litecoin MWEB / HogEx
violates that. A naive merge reintroduces `panic!` / `panic = "abort"` and
a JS client generated **without** `outputs.mweb`.

Read [reference.md](reference.md) for the Litecoin delta and panic surface.

## When invoked

Copy this checklist and work it in order. Do not skip bindgen or the check script.

```
Merge progress:
- [ ] Fetch upstream
- [ ] Merge (no force-push, no wipe of /Volumes/LTC/brk)
- [ ] cargo test --features litecoin (types, rpc, computer)
- [ ] Bindgen WITH litecoin feature
- [ ] Confirm outputs.mweb + transactions.hogex still in client
- [ ] Run scripts/check.sh
- [ ] Restore airbags if the merge dropped them
- [ ] Smoke APIs before swapping the live binary
```

## 1. Fetch and merge

```bash
git remote add upstream https://github.com/bitcoinresearchkit/brk.git 2>/dev/null || true
git fetch upstream
git merge upstream/main
```

If they named another branch (`next`, a tag), use that. Resolve conflicts;
prefer keeping Litecoin saturating math and `catch_unwind` over upstream
`panic!`.

**Never**

- `git push --force` to main
- Delete `/Volumes/LTC/brk` to “fix” the merge
- Run bindgen **without** `--features bindgen,litecoin`
- Commit `docker/cloudflared/credentials.json` or `*.bak-*`

## 2. Tests (laptop, not production data)

```bash
CARGO_TARGET_DIR="$PWD/target" cargo test --features litecoin --lib \
  -p brk_types -p brk_rpc -p brk_computer
```

If a merge restored `panic!` in `Sats::SubAssign` or `FundedAddrData::balance`,
these fail here instead of at height ~2.16M.

## 3. Regenerate clients as Litecoin

```bash
CARGO_TARGET_DIR="$PWD/target" cargo run --example bindgen -p brk_server \
  --features bindgen,litecoin
```

Then `git diff modules/brk-client/index.js` (same inode as
`website/scripts/modules/brk-client/index.js`). **Fail the merge** if
`outputs.mweb` or `transactions.hogex` disappeared.

Do not hand-edit the generated client as the long-term fix; regenerate.

## 4. Run the check script

Execute [scripts/check.sh](scripts/check.sh) from the repo root. Fix anything
it prints, then re-run until exit 0.

```bash
.cursor/skills/merge-upstream/scripts/check.sh
```

## 5. Airbags (do not drop)

If the merge removed any of these, put them back:

| Airbag | Where |
|---|---|
| `panic = "unwind"` | workspace `Cargo.toml` `[profile.release]` |
| `catch_unwind` around `computer.compute` | `crates/brk_cli/src/main.rs` |
| `safeSection(...)` | `website/scripts/options/partial.js` |
| Explorer extras fallback | `crates/brk_query/src/impl/block/info.rs` |
| `--website $REPO/website` | `docker/services/brk.sh` + `install.sh` `BRK_WEBSITE` |

Website in release is **embedded** unless `--website` points at the repo.
Keep filesystem serving so UI fixes do not wait on a rebuild.

## 6. Smoke before live swap

Do **not** run `./docker/services/install.sh` until:

```bash
# after a local release binary is running, or against the still-up old one
curl -sf http://127.0.0.1:7070/health
curl -sf 'http://127.0.0.1:7070/api/series/search?q=mweb' | head -c 200
curl -sf http://127.0.0.1:7070/api/series/mweb_balance/height/len
curl -sf http://127.0.0.1:7070/api/v1/blocks | head -c 200
```

Load `/` and confirm no `TypeError` / `outputs.mweb` crash. Charts → Network → MWEB must exist.

Live deploy (only when asked):

```bash
CARGO_TARGET_DIR="$PWD/target" cargo build --release -p brk_cli --features litecoin
./docker/services/install.sh
```

Always set `CARGO_TARGET_DIR` so `install.sh` copies the binary just built.

## Conflict rules

- **Sats / balances / cost basis / fees / rewards:** keep saturating / `checked_sub` / skip `Sats::MAX` inputs. Upstream `panic!` loses.
- **RPC `getblockchaininfo`:** keep local `BlockchainInfo` that ignores `bip8`.
- **Indexer txout:** `MWEBPegPool` / `MWEBPegIn` stay with unknown-script handling (not `unreachable!`).
- **Investing first price day:** Litecoin `2013-04-01`, not Bitcoin `2010-07-12`.
- **Fee VERSION bumps:** if upstream or we bump `transactions/fees` VERSION, say so — compute will replay; do not wipe the data dir unless the user asks.

## Afterward

Summarize: what upstream brought, which Litecoin patches were reapplied, bindgen result, check.sh output, whether live install is still blocked.
