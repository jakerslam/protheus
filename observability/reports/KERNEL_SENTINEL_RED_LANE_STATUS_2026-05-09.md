# Kernel Sentinel Red Lane Status (2026-05-09)

## Lane boundary

This pass excludes legacy Shell and orchestration implementation work. Red items that require those lanes remain open and untouched.

## Safe red item advanced

`KSENT-RECEIPT-DRIFT` is no longer the active blocker shape. The stale receipt/final-report truth leak is now covered by `ops:ksent:fresh-evidence:guard`, which verifies:

- Sentinel freshness policy exists.
- Sentinel evidence source contains stale-evidence classification hooks.
- Validation has regression coverage for stale generated-at failures.
- Stale Sentinel artifacts cannot be marked authoritative/current `ok:true`.

Current local Sentinel artifacts are stale and `ok:false`, so they cannot be promoted as fresh release truth.

## Remaining live Sentinel blocker

The current live blocker is now `kernel_sentinel_auto_timeout` in `core/local/artifacts/kernel_sentinel_auto_run_current.json`, not receipt drift.

## Follow-up

Track the timeout separately as `KSENT-AUTO-TIMEOUT-FRESH-RUN` so we can debug Sentinel auto-run boundedness without reopening the stale-receipt issue.

## Auto-timeout fresh-run probe

A bounded run was attempted with:

```bash
npm run -s ops:kernel-sentinel:auto -- --max-runtime-ms=30000 --final-report-byte-budget=16000 --final-report-finding-limit=5
```

Result: the command did not enter Sentinel runtime. Rust compilation failed first on an unrelated dashboard compatibility syntax conflict:

```text
core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/045-tool-recovery-and-turn-persistence.rs
error: this file contains an unclosed delimiter
```

Because dashboard compatibility / legacy Shell-adjacent work is outside this thread's allowed lanes, the timeout item remains open and blocked on that compile repair.
