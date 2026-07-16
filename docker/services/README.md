# litview host services (LaunchAgents)

Keeps Litecoin, native BRK, and (on primary) cloudflared running with KeepAlive + a 60s watchdog. Supports an M1 **primary** / M5 **standby** failover pair.

**Why / what changed:** see [RELIABILITY.md](./RELIABILITY.md).

## Roles

| Role | Who | cloudflared |
|------|-----|-------------|
| `primary` (default) | M1 | AlwaysOn LaunchAgent |
| `standby` | M5 | Off until promote (auto or manual) |

Set in the environment or `docker/.env`:

```bash
LITVIEW_ROLE=primary   # or standby
```

## Install

```bash
# Primary (M1)
LITVIEW_ROLE=primary ./docker/services/install.sh

# Standby (M5) — does not load the tunnel agent
LITVIEW_ROLE=standby ./docker/services/install.sh
```

This stops Docker `brk` (`--restart=no`) so native BRK owns `:7070` and `~/.brk`.

On standby, keep `docker/cloudflared/credentials.json` + `config.yml` ready so promote can start the tunnel without copying files during an outage.

## Failover

Standby watchdog probes `https://litview.space/health` (override with `PUBLIC_HEALTH_URL`). After `FAILOVER_FAILURES` consecutive failures (default 3) and local `/health` OK, it runs `promote-standby.sh`.

```bash
./docker/services/promote-standby.sh   # take the tunnel
./docker/services/demote-standby.sh    # release tunnel; stay warm
```

**Failback:** restore M1 tunnel → on M5 run `demote-standby.sh` → confirm public `/health`.

## Uninstall

```bash
./docker/services/uninstall.sh
```

## Logs

`~/Library/Logs/litview/` — `brk.out.log`, `litecoin.out.log`, `cloudflared.*.log`, `watchdog.log`, `failover.log`

## Notes

- Index progress lives in `~/.brk`; stopping/restarting BRK resumes (no full reindex unless BRK detects inconsistency).
- Tunnel ingress must point at `http://127.0.0.1:7070`.
- Rebuild BRK after code changes: `cargo build --release -p brk_cli --features litecoin` then restart the agent when idle.
- Snapshot `~/.brk` to external disk when available (see RELIABILITY.md remaining work).
- Standby only helps if its index is near tip and disk has headroom.
