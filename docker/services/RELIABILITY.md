# Making litview.space more robust

This documents the reliability changes for the litview hosts after a multi-hour outage in July 2026. Goal: **self-heal process crashes and tunnel drops**, with a **warm standby** on a second Mac. This is not multi-region five-nines.

Related ops: [README.md](./README.md) (install/uninstall), [../cloudflared/README.md](../cloudflared/README.md) (tunnel).

---

## Roles (M1 primary / M5 standby)

| Host | Role | Runs |
|------|------|------|
| M1 MacBook Pro | `LITVIEW_ROLE=primary` | Litecoin + brk + **cloudflared AlwaysOn** + watchdog |
| M5 MacBook Pro | `LITVIEW_ROLE=standby` | Litecoin + brk + watchdog; **cloudflared only when promoted** |

Both use the **same** Cloudflare tunnel UUID and `docker/cloudflared/config.yml` → `http://127.0.0.1:7070`. Each machine keeps its **own** Litecoin datadir and `~/.brk` (not shared).

```text
Visitors → Cloudflare → same tunnel UUID
                │
                ├─ normal: M1 cloudflared → M1 brk :7070
                └─ failover: M5 cloudflared (promoted) → M5 brk :7070
```

Do **not** leave cloudflared KeepAlive on both hosts at once in normal operation — Cloudflare would treat them as replicas and may send traffic to either.

### Install per role

```bash
# M1 (primary) — default
LITVIEW_ROLE=primary ./docker/services/install.sh
# or set LITVIEW_ROLE=primary in docker/.env then:
./docker/services/install.sh

# M5 (standby) — tunnel plist installed but not loaded
LITVIEW_ROLE=standby ./docker/services/install.sh
```

Ensure M5 has `docker/cloudflared/credentials.json` + matching `config.yml` before an outage (promote cannot copy secrets mid-failure).

### Automatic failover (standby)

Watchdog on standby (every 60s):

1. Probe `PUBLIC_HEALTH_URL` (default `https://litview.space/health`)
2. After **`FAILOVER_FAILURES`** consecutive failures (default **3**) **and** local `http://127.0.0.1:7070/health` is OK → run `promote-standby.sh`
3. Does **not** auto-demote (avoids flapping)

Logs: `~/Library/Logs/litview/failover.log`, `watchdog.log`

### Manual promote / demote

```bash
./docker/services/promote-standby.sh   # start cloudflared on this host (local /health required)
./docker/services/demote-standby.sh    # stop cloudflared; keep Litecoin+brk warm
```

### Failback checklist (M1 recovered)

1. Confirm M1 litecoin + brk healthy: `curl -sf http://127.0.0.1:7070/health`
2. Confirm M1 tunnel agent is loaded/running (`com.litview.cloudflared`)
3. On **M5**: `./docker/services/demote-standby.sh`
4. Verify public still OK: `curl -sf https://litview.space/health`

### Dry-run

1. On M1: `launchctl bootout gui/$(id -u)/com.litview.cloudflared` (or demote if testing the other way)
2. Within ~3 minutes M5 watchdog should promote
3. Public `/health` returns 200 via M5
4. Demote M5; restore M1 tunnel; confirm public still 200

---

## What broke (incident summary)

Public site (`litview.space`) returned Cloudflare **502** while the tunnel process was often still up. The failure chain was:

1. **Litecoin Core write failure** (disk I/O), then shutdown:
   ```text
   ERROR: ProcessNewBlock: AcceptBlock FAILED
   (System error: CAutoFile::write: write failed: unspecified iostream_category error)
   ```
   Data volume was ~91% full (~40 GB free at the time; later dipped to single-digit GB during reindex).

2. **Docker Desktop wedged** — host `:7070` still listened, but connections reset. Docker API hung (`docker ps` / sock ping timed out).

3. **cloudflared** correctly reported `connection reset by peer` / `connection refused` to the origin.

4. After recovery, **BRK** saw a tip reorg / index inconsistency and ran a **full indexer reset** (hours offline while HTTP stayed closed when far behind tip).

Separately, the heatmap URL units were renamed (`sats`/`btc` → `lits`/`ltc`) and the tunnel ingress was corrected from `:3110` to `:7070` (Docker maps host `7070` → container `3110`; native BRK now listens on `7070` directly).

A later tip stall was caused by a **corrupt duplicate block** in `blk*.dat` aborting the reader before a good copy; the reader now skips corrupt duplicates, and `/health` no longer panics the process on sync errors.

---

## Before vs after (single host)

### Before

```text
Visitors → Cloudflare → cloudflared (manual ./start.sh in a terminal)
                              → localhost:7070 → Docker Desktop → brk container
Litecoin-Qt (GUI, no auto-restart)
```

### After (primary)

```text
Visitors → Cloudflare → cloudflared (LaunchAgent KeepAlive)
                              → localhost:7070 → native brk (LaunchAgent KeepAlive)
Litecoin-Qt -daemon (LaunchAgent KeepAlive wrapper)
Watchdog every 60s → kickstarts dead jobs; warns on low disk
```

Docker `brk` is **stopped** with `restart=no` so it does not fight native BRK for `:7070` / `~/.brk`.

---

## What we changed

### 1. Litecoin as a supervised service

- **Files:** `litecoin.sh`, `com.litview.litecoin.plist`
- Runs Litecoin-Qt with `-daemon`, then **waits on the pid** so launchd `KeepAlive` restarts after exit (including disk-write crashes).
- Adopts an already-running node if present.

### 2. Native BRK instead of Docker Desktop

- **Files:** `brk.sh`, `com.litview.brk.plist`
- Uses repo binary: `target/release/brk` built with `--features litecoin`
- Listens on **`--brkport 7070`** (matches tunnel ingress)
- Data: `~/.brk` (same tree Docker was bind-mounting)
- Chain data: `~/Library/Application Support/Litecoin` (read via `--bitcoindir` / `--blocksdir`)
- RPC settings remain in `~/.brk/config.toml` (and/or env)

**Why:** removes Docker Desktop from the serving path. Process crash → launchd restart → index **resumes** from `~/.brk` when consistent (full reset only if BRK detects inconsistency).

### 3. cloudflared as a LaunchAgent

- **Files:** `com.litview.cloudflared.plist`
- Same `docker/cloudflared/config.yml` (ingress → `http://127.0.0.1:7070`)
- Survives closed terminals / Cursor sessions; restarts on crash
- On **standby**, plist is installed but not loaded until promote

### 4. Watchdog

- **Files:** `watchdog.sh`, `com.litview.watchdog.plist` (`StartInterval` = 60s)
- Checks:
  - Litecoin RPC (`getblockcount` on `:9332`) → kickstart `com.litview.litecoin` if down
  - BRK process if `/health` fails → kickstart `com.litview.brk` only when `brk` is not running (avoids killing a healthy indexer mid-sync)
  - **Primary:** `cloudflared tunnel` process → kickstart tunnel agent
  - **Standby:** public `/health` failures → `promote-standby.sh` (no tunnel kickstart while demoted)
  - Free space on `/System/Volumes/Data` → **WARN** if below 50 GB (no automatic delete)

Logs: `~/Library/Logs/litview/watchdog.log`

### 5. Install / uninstall / failover scripts

- `install.sh` — role-aware; copies plists; primary loads tunnel, standby does not
- `uninstall.sh` — boots out agents and removes plists
- `promote-standby.sh` / `demote-standby.sh` — tunnel ownership for standby

---

## How this improves reliability

| Failure mode | Old behavior | New behavior |
|--------------|--------------|--------------|
| Litecoin crash | Site dead until manual relaunch | KeepAlive / watchdog restarts node |
| `brk` exit / panic | Depends on Docker + human | KeepAlive restarts native binary |
| Tunnel terminal closed | Immediate Cloudflare errors | LaunchAgent keeps tunnel up |
| Docker Desktop freeze | Hard to diagnose; API hung | Not on the request path anymore |
| Disk filling up | Silent until write failures | Watchdog warns (&lt; 50 GB) |
| Primary host down | Site dead | Standby auto-promotes tunnel (~3 min) |

**Still out of scope:** power/sleep/ISP on both hosts at once, APFS nearly-full write failures, and **full index resets** after inconsistency (multi-hour HTTP blackout on that host). Keep disk headroom; snapshot `~/.brk` when possible. Standby only helps if its index is near tip.

---

## Operating notes

```bash
# Install / refresh agents (primary)
LITVIEW_ROLE=primary ./docker/services/install.sh

# Follow BRK
tail -f ~/Library/Logs/litview/brk.out.log

# Status
launchctl print gui/$(id -u)/com.litview.brk | head
curl -sf http://127.0.0.1:7070/health && echo OK

# After rebuilding BRK
cargo build --release -p brk_cli --features litecoin
# Prefer: stop gracefully when idle, then launchctl start com.litview.brk
# Avoid kickstart -k while indexing — it waits on process exit
```

**Do not** `docker compose up brk` while native BRK is serving — port and data dir conflict.

**Pause indexing safely:** stop the BRK agent (or `kill` the `brk` pid); `~/.brk` keeps flushed progress. Resume by starting `com.litview.brk` again.

---

## Remaining work (not done yet)

1. **Free disk / external SSD** — keep substantial free space; Litecoin + `~/.brk` are hundreds of GB on one volume.
2. **Snapshot `~/.brk`** to external storage when available so a full reset is not the only recovery path.
3. Optional later: Cloudflare Load Balancer (true weighted primary), Linux/`litecoind` instead of Litecoin-Qt.

---

## File map

| Path | Role |
|------|------|
| `litecoin.sh` / `com.litview.litecoin.plist` | Supervised Litecoin |
| `brk.sh` / `com.litview.brk.plist` | Native BRK on `:7070` |
| `com.litview.cloudflared.plist` | Supervised tunnel (primary AlwaysOn) |
| `watchdog.sh` / `com.litview.watchdog.plist` | 60s health + disk warn + standby promote |
| `install.sh` / `uninstall.sh` | Load / unload agents (role-aware) |
| `promote-standby.sh` / `demote-standby.sh` | Tunnel failover helpers |
| `../cloudflared/config.yml` | Tunnel → `127.0.0.1:7070` |
| `../.env` | `LITVIEW_ROLE`, RPC, failover knobs |
| `~/Library/Logs/litview/` | Runtime + failover logs |
| `~/Library/LaunchAgents/com.litview.*.plist` | Installed agents |
