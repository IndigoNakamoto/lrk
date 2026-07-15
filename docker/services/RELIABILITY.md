# Making litview.space more robust (this Mac)

This documents the reliability changes made for the litview demo host after a multi-hour outage in July 2026. Goal: **self-heal process crashes and tunnel drops** on a single machine. This is not multi-host HA / five-nines.

Related ops: [README.md](./README.md) (install/uninstall), [../cloudflared/README.md](../cloudflared/README.md) (tunnel).

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

---

## Before vs after

### Before

```text
Visitors → Cloudflare → cloudflared (manual ./start.sh in a terminal)
                              → localhost:7070 → Docker Desktop → brk container
Litecoin-Qt (GUI, no auto-restart)
```

Single points of failure:

| Piece | Risk |
|-------|------|
| Litecoin-Qt | Crash stayed down until someone relaunched |
| Docker Desktop | Engine freeze = site dead even if “port open” |
| `./start.sh` | Closing the terminal killed the tunnel |
| Healthcheck | Container `pgrep brk` ≠ HTTP healthy |
| Disk | No ongoing free-space alarm |

### After

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
- Demo `./start.sh` is optional; prefer the agent for always-on

### 4. Watchdog

- **Files:** `watchdog.sh`, `com.litview.watchdog.plist` (`StartInterval` = 60s)
- Checks:
  - Litecoin RPC (`getblockcount` on `:9332`) → kickstart `com.litview.litecoin` if down
  - BRK process if `/health` fails → kickstart `com.litview.brk` only when `brk` is not running (avoids killing a healthy indexer mid-sync)
  - `cloudflared tunnel` process → kickstart tunnel agent
  - Free space on `/System/Volumes/Data` → **WARN** if below 50 GB (no automatic delete)

Logs: `~/Library/Logs/litview/watchdog.log`

### 5. Install / uninstall

- `install.sh` — copies plists to `~/Library/LaunchAgents`, bootstraps agents, stops Docker `brk`
- `uninstall.sh` — boots out agents and removes plists

---

## How this improves reliability

| Failure mode | Old behavior | New behavior |
|--------------|--------------|--------------|
| Litecoin crash | Site dead until manual relaunch | KeepAlive / watchdog restarts node |
| `brk` exit / panic | Depends on Docker + human | KeepAlive restarts native binary |
| Tunnel terminal closed | Immediate Cloudflare errors | LaunchAgent keeps tunnel up |
| Docker Desktop freeze | Hard to diagnose; API hung | Not on the request path anymore |
| Disk filling up | Silent until write failures | Watchdog warns (&lt; 50 GB) |

**Still out of scope on one Mac:** power/sleep/ISP, APFS nearly-full write failures, and **full index resets** after inconsistency (multi-hour HTTP blackout). Those need disk headroom, external backups of `~/.brk`, and eventually a second host.

---

## Operating notes

```bash
# Install / refresh agents
./docker/services/install.sh

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
3. **Optional later:** second machine + shared Cloudflare tunnel connectors; Linux/`litecoind` instead of Litecoin-Qt; fix BRK health-task panic (`Internal("data unavailable")`) so a tip reorg does not always force a full reset after a crash.

---

## File map

| Path | Role |
|------|------|
| `litecoin.sh` / `com.litview.litecoin.plist` | Supervised Litecoin |
| `brk.sh` / `com.litview.brk.plist` | Native BRK on `:7070` |
| `com.litview.cloudflared.plist` | Supervised tunnel |
| `watchdog.sh` / `com.litview.watchdog.plist` | 60s health + disk warn |
| `install.sh` / `uninstall.sh` | Load / unload agents |
| `../cloudflared/config.yml` | Tunnel → `127.0.0.1:7070` |
| `../.env` | RPC creds for watchdog (`CHAIN_DATA_DIR` quoted) |
| `~/Library/Logs/litview/` | Runtime logs |
| `~/Library/LaunchAgents/com.litview.*.plist` | Installed agents |
