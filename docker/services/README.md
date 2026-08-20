# litview host services (LaunchAgents)

Keeps Litecoin, native BRK, and cloudflared running with KeepAlive + a 60s watchdog.

**Why / what changed:** see [RELIABILITY.md](./RELIABILITY.md).

## Install

```bash
# Optional: docker/.env with CHAIN_DATA_DIR / BRK_DATA_DIR on an external SSD
./docker/services/install.sh
```

This copies scripts + the `brk` binary to `~/Library/Application Support/litview` (so launchd can exec them), stops Docker `brk`, and loads agents.

**External SSD / Removable Volume:** set `CHAIN_DATA_DIR` and `BRK_DATA_DIR` in `docker/.env`. macOS LaunchAgents cannot write to `/Volumes/...` without Full Disk Access, so `com.litview.litecoin` is skipped in that case — start the node from Terminal:

```bash
~/Library/Application\ Support/litview/start-litecoin.sh
```

## Deploy a new BRK binary (production)

Cursor / some shells set `CARGO_TARGET_DIR` to a sandbox cache. Always build into the repo `target/` so `install.sh` copies the binary you just built:

```bash
cd /Volumes/LTC/lrk
CARGO_TARGET_DIR=$PWD/target cargo build --release -p brk_cli --features litecoin
./docker/services/install.sh
```

Confirm the installed binary matches:

```bash
ls -la target/release/brk \
  "$HOME/Library/Application Support/litview/bin/brk"
curl -sS http://127.0.0.1:7070/health
curl -sS http://127.0.0.1:7070/api/v1/mining/hashrate/3d | head -c 200
```

Data dirs on this host: `BRK_DATA_DIR=/Volumes/LTC/brk`, chain under `/Volumes/LTC/litecoin`.

## Tunnel (litview.space)

Ingress must point at `http://127.0.0.1:7070`. Config lives in `docker/cloudflared/` (LaunchAgent working directory).

Production DNS is routed to tunnel **`litview-demo`** (`7df1ff7a-…`) with credentials only on this Mac. The older **`litview-m1`** tunnel may still have a secondary connector online — it must not receive `litview.space` DNS, or public traffic can stick on a stale BRK.

After changing the tunnel binary/config:

```bash
launchctl kickstart -k "gui/$(id -u)/com.litview.cloudflared"
cloudflared tunnel info litview-demo   # needs ~/.cloudflared/cert.pem from `cloudflared tunnel login`
curl -sS https://litview.space/api/v1/mining/hashrate/3d | head -c 200
```

Do **not** run `sudo cloudflared service install` on this Mac (other tunnels use the system daemon). Do **not** re-run `route-dns.sh` unless DNS is wrong.

## Uninstall

```bash
./docker/services/uninstall.sh
```

## Logs

`~/Library/Logs/litview/` — `brk.out.log`, `litecoin.out.log`, `cloudflared.*.log`, `watchdog.log`

## Notes

- Index progress lives under `BRK_DATA_DIR` (here `/Volumes/LTC/brk`); stopping/restarting BRK resumes when consistent.
- Snapshot index data to external disk when available (see RELIABILITY.md remaining work).
