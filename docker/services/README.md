# litview host services (LaunchAgents)

Keeps Litecoin, native BRK, and cloudflared running with KeepAlive + a 60s watchdog.

**Why / what changed:** see [RELIABILITY.md](./RELIABILITY.md).

## Install

```bash
./docker/services/install.sh
```

This stops Docker `brk` (`--restart=no`) so native BRK owns `:7070` and `~/.brk`.

## Uninstall

```bash
./docker/services/uninstall.sh
```

## Logs

`~/Library/Logs/litview/` — `brk.out.log`, `litecoin.out.log`, `cloudflared.*.log`, `watchdog.log`

## Notes

- Index progress lives in `~/.brk`; stopping/restarting BRK resumes (no full reindex unless BRK detects inconsistency).
- Tunnel ingress must point at `http://127.0.0.1:7070`.
- Rebuild BRK after code changes: `cargo build --release -p brk_cli --features litecoin` then restart the agent when idle.
- Snapshot `~/.brk` to external disk when available (see RELIABILITY.md remaining work).
