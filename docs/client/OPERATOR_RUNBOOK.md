# Operator Runbook

Purpose: deterministic incident response for autonomy/routing/sensory failures with auditable verification.

## Scope

Covers:

1. Routing degraded
2. Schema drift / contract failure
3. Sensory starvation
4. Autonomy stall
5. Secure heartbeat endpoint compromise / drift

## Global Triage

1. Capture health snapshot:
`node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. If unsafe behavior is live, engage kill switch first:
`node client/runtime/systems/security/emergency_stop.ts engage --scope=all --approval-note="contain incident"`
3. Run core contract guards:
`node client/runtime/systems/spine/contract_check_bridge.ts`
`node client/runtime/systems/security/schema_contract_check.ts run`
`node client/runtime/systems/sensory/adaptive_layer_guard.ts run --strict`

Expected artifacts:

- `state/security/emergency_stop.json`
- `state/autonomy/runs/YYYY-MM-DD.jsonl`
- `state/autonomy/receipts/YYYY-MM-DD.jsonl`

## First-Run Failure Decision Tree (Install/Setup/Gateway)

Use this deterministic path for first-run issues before deeper incident lanes:

1. `infring` command not found
- Reload PATH:
` . "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true`
- Verify:
`infring --help`
- Direct-path fallback:
`"$HOME/.infring/bin/infring" --help`

2. Setup not completed / onboarding pending
- Run:
`infring setup --yes --defaults`
- Verify:
`infring setup status --json`

3. Gateway/dashboard unavailable
- Check:
`infring gateway status`
- Restart:
`infring gateway restart`
- Verify health:
`curl -fsS http://127.0.0.1:4173/healthz`

4. Stale root/path drift
- Diagnose:
`infring doctor --json`
- Confirm active root aligns with `INFRING_WORKSPACE_ROOT` (and compatibility alias `PROTHEUS_WORKSPACE_ROOT`) for this workspace.

5. Full command surface unavailable due to missing Node
- Reinstall full with Node bootstrap:
`curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node`
- If full surface is not required, use constrained runtime mode (`--pure` / `--tiny-max`).

### Deterministic Failure-Code Mapping

| failure_code | check | expected output | immediate recovery |
| --- | --- | --- | --- |
| `command_not_found` | `infring --help` | Canonical wrapper help output | Reload env, then run direct wrapper path |
| `setup_incomplete` | `infring setup status --json` | `onboarding_receipt.status` = `incomplete` plus mode/workspace | `infring setup --yes --defaults`, then re-check status |
| `gateway_unhealthy` | `infring gateway status` + `/healthz` | Deterministic gateway contract + health endpoint response | `infring gateway restart`, then verify `/healthz` |
| `stale_workspace_root` | `infring doctor --json` | Root/path drift fields explicitly present | Align workspace root env vars and rerun doctor |
| `full_surface_dependency_missing` | Full install with `--install-node` | Runtime wrappers and full-surface dependencies installed | Retry onboarding bootstrap for current role |

## Failure Artifact Quick Reference

Capture these artifacts on first-run failures:

- Onboarding receipts:
  - `local/state/ops/onboarding_portal/bootstrap_<role>.json`
  - `local/state/ops/onboarding_portal/bootstrap_<role>.txt`
  - `local/state/ops/onboarding_portal/bootstrap_<role>_failure_snapshot.json` (on failure)
- Setup wizard state:
  - `local/state/ops/protheus_setup_wizard/latest.json`
  - `local/state/ops/first_run_onboarding_wizard/latest.json`
- Installer/runtime logs:
  - `$HOME/.infring/logs/dashboard_ui.log`
  - `$HOME/.infring/logs/dashboard_watchdog.log`
- Recovery commands to record:
  - `infring gateway status`
  - `infring gateway restart`
  - `infring doctor --json`
  - `curl -fsS http://127.0.0.1:4173/healthz`

## Repair Evidence Checklist (Deterministic)

Use this checklist before and after repair actions so incidents have a complete artifact chain.

1. Capture pre-repair machine state:
`infring doctor --json > local/state/ops/repair_evidence/doctor_pre.json`
2. Capture setup/runtime state:
`infring setup status --json > local/state/ops/repair_evidence/setup_status_pre.json`
3. Capture gateway state:
`infring gateway status > local/state/ops/repair_evidence/gateway_status_pre.txt`
4. Execute repair chain:
`infring gateway restart`
5. Capture post-repair machine state:
`infring doctor --json > local/state/ops/repair_evidence/doctor_post.json`
6. Capture post-repair gateway health:
`curl -fsS http://127.0.0.1:4173/healthz > local/state/ops/repair_evidence/healthz_post.txt`

Expected artifact IDs per repair:

- `doctor_pre.json`
- `setup_status_pre.json`
- `gateway_status_pre.txt`
- `doctor_post.json`
- `healthz_post.txt`

Common breakage states and required evidence:

1. `command_not_found`:
- PATH resolution proof (`infring --help` output, direct wrapper fallback output)
2. `setup_incomplete`:
- `infring setup status --json` pre/post
3. `gateway_unhealthy`:
- `infring gateway status` pre/post and `/healthz` post-restart
4. `stale_workspace_root`:
- `infring doctor --json` root/path findings pre/post

Canonical recovery-mode aliases (for test/automation parity):

1. `node_runtime_missing`:
- Alias of `command_not_found` in first-run recovery context.
2. `dashboard_down`:
- Alias of `gateway_unhealthy` (requires gateway status + `/healthz` evidence).
3. `stale_launch_artifact`:
- Alias of `stale_workspace_root` (requires doctor pre/post root-path evidence).

## Incident 1: Routing Degraded

Symptoms:

- `health_status.routing.spine_local_down_consecutive > 0`
- repeated `route_blocked` or stale/local-down in routing decisions

Diagnose:

1. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. `node client/runtime/systems/routing/model_router.ts doctor --risk=low --complexity=low --intent=ops_triage --task="routing incident diagnosis"`
3. `node client/runtime/systems/routing/model_router.ts probe-all`
4. `node client/runtime/systems/routing/model_router.ts stats`

Containment:

- Optional scope-limited stop while recovering:
`node client/runtime/systems/security/emergency_stop.ts engage --scope=routing --approval-note="routing containment"`

Recovery:

1. Unban false positives if needed:
`node client/runtime/systems/routing/model_router.ts unban --model=ollama/<name>`
2. Force safe execution mode if routing quality is uncertain:
`node client/runtime/systems/autonomy/strategy_mode.ts set --mode=score_only --approval-note="routing degraded; reduce blast radius"`

Verification:

1. Re-run doctor command (step 2).
2. Confirm no new `route_blocked` spikes in:
`state/routing/routing_decisions.jsonl`
3. Confirm health status routing section is stable.

## Incident 2: Schema Drift / Contract Failure

Symptoms:

- `schema_contract_check` fails
- `contract_check` fails
- receipt/proposal fields missing or malformed

Diagnose:

1. `node client/runtime/systems/security/schema_contract_check.ts run`
2. `node client/runtime/systems/spine/contract_check_bridge.ts`
3. `node client/runtime/systems/sensory/adaptive_layer_guard.ts run --strict`

Containment:

`node client/runtime/systems/security/emergency_stop.ts engage --scope=autonomy,routing,actuation --approval-note="schema drift containment"`

Recovery:

1. Identify recent code deltas:
`git log --oneline -n 20`
2. Revert offending commit(s) non-destructively:
`git revert --no-edit <commit_sha>`
3. Re-run diagnosis commands.

Verification:

- all three checks above pass
- no new contract failures in CI run:
`npm run test:ci`

## Incident 3: Sensory Starvation

Symptoms:

- low or zero external signal intake
- high collector error ratio
- sparse/no new actionable proposals

Diagnose:

1. `node client/cognition/habits/scripts/external_eyes.ts doctor`
2. `node client/cognition/habits/scripts/external_eyes.ts slo [YYYY-MM-DD]`
3. `node client/runtime/systems/spine/spine.ts eyes [YYYY-MM-DD] --max-eyes=3`
4. `node client/runtime/systems/autonomy/proposal_enricher.ts run [YYYY-MM-DD] --dry-run`

Containment:

- Keep autonomy in safer mode while sensory is degraded:
`node client/runtime/systems/autonomy/strategy_mode.ts set --mode=score_only --approval-note="sensory degraded; hold execution risk"`

Recovery:

1. Refresh focus triggers:
`node client/runtime/systems/sensory/focus_controller.ts refresh [YYYY-MM-DD]`
2. Resolve collector-specific failures indicated by doctor/slo output.

Verification:

1. `external_eyes.ts slo` returns healthy.
2. `state/sensory/proposals/YYYY-MM-DD.json` contains current-day actionable records.
3. `health_status` sensory metrics recover.

## Incident 4: Autonomy Stall

Symptoms:

- repeated stop/repeat gate outcomes
- low/no executed outcomes over multiple runs
- readiness/governor blocks persist

Diagnose:

1. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. `node client/runtime/systems/autonomy/strategy_readiness.ts run [YYYY-MM-DD] --days=14`
3. `node client/runtime/systems/autonomy/pipeline_spc_gate.ts run [YYYY-MM-DD] --days=1 --baseline-days=7 --sigma=3`
4. `node client/runtime/systems/autonomy/receipt_summary.ts run [YYYY-MM-DD] --days=7`
5. `node client/runtime/systems/autonomy/strategy_mode_governor.ts status [YYYY-MM-DD] --days=14`

Containment:

- keep/return to score-only until pass rates recover:
`node client/runtime/systems/autonomy/strategy_mode.ts set --mode=score_only --approval-note="autonomy stall containment"`

Recovery:

1. Recompute admission metadata:
`node client/runtime/systems/autonomy/proposal_enricher.ts run [YYYY-MM-DD]`
2. Run governor in dry mode to inspect proposed transition:
`node client/runtime/systems/autonomy/strategy_mode_governor.ts run [YYYY-MM-DD] --days=14 --dry-run`

Verification:

- `strategy_readiness` returns `ready_for_execute=true` before enabling execute/canary
- receipt pass rates and success criteria pass rates trend up in `receipt_summary`

## Incident 5: Queue Backlog / Churn

Symptoms:

- OPEN queue count grows daily
- repeated reject noise in queue logs
- stale proposals never resolve

Diagnose:

1. `node client/cognition/habits/scripts/sensory_queue.ts stats --days=7`
2. `node client/cognition/habits/scripts/sensory_queue.ts list --status=open --days=7`
3. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]` (check queue backlog SLO + recovery pulse)

Containment / Recovery:

1. Run deterministic hygiene:
`node client/cognition/habits/scripts/queue_gc.ts run [YYYY-MM-DD]`
`node client/cognition/habits/scripts/sensory_queue.ts sweep [YYYY-MM-DD]`
2. Force compact terminal churn:
`node client/runtime/systems/ops/queue_log_compact.ts run --apply=1`
3. If backlog is budget-constrained, pin pressure explicitly for a run:
`QUEUE_GC_BUDGET_PRESSURE=hard node client/cognition/habits/scripts/queue_gc.ts run [YYYY-MM-DD]`

Verification:

1. queue open count trends down in `sensory_queue stats`
2. no repeated reject spam for same id/reason in `state/sensory/queue_log.jsonl`
3. `health_status` queue backlog check returns pass

## Incident 6: Dream Degradation

Symptoms:

- `health_status` reports `dream_degradation` warn/critical
- repeated idle/REM fallback usage with low synthesis quality
- dream model cooldown churn (timeouts/rate limits)

Diagnose:

1. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. `node client/runtime/systems/memory/idle_dream_cycle.ts status`
3. `node client/runtime/systems/memory/idle_dream_cycle.ts run [YYYY-MM-DD] --force=1`

Containment / Recovery:

1. If repeated cloud/local timeout loops occur, keep dream lane degraded until stable:
`IDLE_DREAM_FORCE_DEGRADED=1 node client/runtime/systems/memory/idle_dream_cycle.ts run [YYYY-MM-DD] --force=1`
2. Reduce dream pressure during instability:
`IDLE_DREAM_MAX_ITEMS=6 node client/runtime/systems/memory/idle_dream_cycle.ts run [YYYY-MM-DD] --force=1`
3. Restore normal lane after two stable cycles.

Verification:

1. `health_status` dream degradation returns pass.
2. Dream outputs include non-fallback synthesis rows.
3. Timeout/cooldown events no longer trend upward.

## Incident 7: Budget Pressure / Autopause

Symptoms:

- `health_status` reports `budget_pressure` warn/critical
- `budget_autopause_active=true` in health gates
- execution lanes stop with budget/autopause gate reasons

Diagnose:

1. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. `node client/runtime/systems/budget/system_budget.ts status`
3. `node client/runtime/systems/budget/system_budget.ts tail --limit=50`

Containment / Recovery:

1. Keep autonomy in score-only while pressure is unresolved:
`node client/runtime/systems/autonomy/strategy_mode.ts set --mode=score_only --approval-note="budget pressure containment"`
2. Allow autopause window to expire or clear with approved operator action.
3. Lower non-critical lane usage before re-enabling execute/canary.

Verification:

1. `budget_pressure` check returns pass in `health_status`.
2. New budget decisions are predominantly `allow`.
3. Autopause no longer active.

## Incident 8: Verification Pass-Rate Regression

Symptoms:

- `health_status` reports `verification_pass_rate` warn/critical
- `receipt_summary` shows falling `receipts.combined.verified_rate`
- repeated failure reasons in autonomy/actuation receipts (timeouts, rate limits, rollback-triggered failures)

Diagnose:

1. `node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. `node client/runtime/systems/autonomy/receipt_summary.ts run [YYYY-MM-DD] --days=7`
3. `node client/runtime/systems/autonomy/slo_runbook_check.ts run [YYYY-MM-DD]`
4. Inspect top reasons:
`jq '.receipts.combined.top_failure_reasons' state/autonomy/health_client/reports/[YYYY-MM-DD].daily.json`

Containment / Recovery:

1. Keep execution bounded while quality recovers:
`node client/runtime/systems/autonomy/strategy_mode.ts set --mode=score_only --approval-note="verification pass-rate containment"`
2. Prioritize failure-class remediation from top failure reasons (timeout/rate-limit/rollback).
3. Re-run targeted integration tests for failing lane:
`node tests/client-memory-tools/pipeline_handoffs.integration.test.ts`

Verification:

1. `verification_pass_rate` check returns pass in `health_status`.
2. `receipt_summary` verified-rate trend is stable or improving.
3. Top failure-reason concentration drops over the last 7-day window.

## Rollback Drill (Weekly)

Goal: verify rollback muscle memory and logging path.

1. Snapshot status:
`node client/runtime/systems/autonomy/health_status.ts [YYYY-MM-DD]`
2. Simulate change containment:
`node client/runtime/systems/security/emergency_stop.ts engage --scope=autonomy --approval-note="rollback drill"`
3. Run core guards:
`node client/runtime/systems/security/schema_contract_check.ts run`
`node client/runtime/systems/sensory/adaptive_layer_guard.ts run --strict`
4. Release stop:
`node client/runtime/systems/security/emergency_stop.ts release --approval-note="rollback drill complete"`
5. Record drill outcome in commit/ops notes.

## Verification Logs and Receipts

Primary files to inspect per incident:

- `state/security/emergency_stop.json`
- `state/security/emergency_stop_events.jsonl`
- `state/security/adaptive_mutations.jsonl`
- `state/autonomy/runs/YYYY-MM-DD.jsonl`
- `state/autonomy/receipts/YYYY-MM-DD.jsonl`
- `state/routing/routing_decisions.jsonl`
- `state/ops/offsite_backup_sync_receipts.jsonl`
- `state/ops/offsite_restore_drill_receipts.jsonl`

## Incident 9: Offsite DR Gap / Restore Drill Overdue

Symptoms:

- `offsite_backup status` reports `restore_drill_due=true`
- latest offsite sync or restore drill has `ok=false`
- no recent offsite receipts despite local state backups

Diagnose:

1. `node client/runtime/systems/ops/offsite_backup.ts status`
2. `node client/runtime/systems/ops/offsite_backup.ts list --limit=3`
3. `tail -n 20 state/ops/offsite_backup_sync_receipts.jsonl`
4. `tail -n 20 state/ops/offsite_restore_drill_receipts.jsonl`

Containment / Recovery:

1. Ensure encryption key + destinations are set:
`echo $STATE_BACKUP_OFFSITE_KEY | wc -c`
`echo $STATE_BACKUP_OFFSITE_DEST`
2. Run strict sync:
`node client/runtime/systems/ops/offsite_backup.ts sync --strict=1`
3. Run strict restore drill:
`node client/runtime/systems/ops/offsite_backup.ts restore-drill --strict=1`

Verification:

1. `offsite_backup status` returns `restore_drill_due=false`.
2. Latest sync receipt has `ok=true`.
3. Latest restore-drill receipt has `ok=true` with `metrics.rto_minutes` and `metrics.rpo_hours` present.

## Incident 10: Secure Heartbeat Endpoint Drift/Compromise

Symptoms:

- repeated heartbeat deny events (`signature_mismatch`, `key_not_found`, `rate_limited`)
- secure heartbeat verify fails
- endpoint receives traffic but `latest_heartbeat_id` stops advancing

Diagnose:

1. `node client/runtime/systems/security/secure_heartbeat_endpoint.ts verify --strict=1`
2. `node client/runtime/systems/security/secure_heartbeat_endpoint.ts status`
3. `tail -n 30 state/security/secure_heartbeat_endpoint/audit.jsonl`
4. `tail -n 30 state/security/secure_heartbeat_endpoint/alerts.jsonl`

Containment / Recovery:

1. Revoke suspicious key:
`node client/runtime/systems/security/secure_heartbeat_endpoint.ts revoke-key --key-id=<id> --reason="incident_response"`
2. Rotate client key:
`node client/runtime/systems/security/secure_heartbeat_endpoint.ts issue-key --client-id=<client>`
3. If endpoint behavior is uncertain, keep remote channels advisory-only until audit stabilizes.

Verification:

1. `verify --strict=1` passes.
2. deny-rate in audit stream drops to baseline.
3. latest heartbeat advances with accepted signed payloads.

## Incident 11: Cron Delivery Misconfiguration

Symptoms:

- `health_status` reports `cron_delivery_integrity` warn/critical
- cron jobs fail silently or route to invalid channels
- isolated cron jobs missing `delivery` config

Diagnose:

1. `protheus-ops status --dashboard`
2. `protheus-ops health-status status`
3. Inspect cron definitions:
`cat client/runtime/config/cron_jobs.json`

Containment / Recovery:

1. Remove/replace any `delivery.mode: \"none\"` entries for enabled jobs.
2. For isolated jobs, enforce:
`\"delivery\": { \"mode\": \"announce\", \"channel\": \"last\" }`
3. Normalize invalid channels to an approved channel:
`last|main|inbox|discord|slack|email|pagerduty|stdout|stderr|sms`

Verification:

1. `protheus-ops status --dashboard` returns `cron_delivery_integrity.status=pass`.
2. `health_status` alert list no longer includes `cron_delivery_integrity`.
3. Latest health receipt hash is present.

## Incident 12: Rust Source-of-Truth Drift

Symptoms:

- `health_status` reports `rust_source_of_truth` warn/critical
- `contract-check` fails on required Rust/TS/JS boundary tokens
- TS entrypoints diverge from Rust authoritative command routes

Diagnose:

1. `protheus-ops contract-check`
2. `protheus-ops status --dashboard`
3. Inspect policy:
`cat client/runtime/config/rust_source_of_truth_policy.json`

Containment / Recovery:

1. Restore required gate tokens in:
   - `core/layer0/ops/src/main.rs`
   - `core/layer2/conduit/src/lib.rs`
   - `client/runtime/systems/ops/protheusd.ts`
   - `client/runtime/systems/ops/protheus_status_dashboard.ts`
2. Ensure wrappers remain wrappers:
   - `.js` wrappers use `ts_bootstrap`
   - Rust shims remain `.js` spawn bridges only
3. Re-run contract and health checks until both are green.

Verification:

1. `protheus-ops contract-check` exits `0`.
2. `protheus-ops status --dashboard` returns `rust_source_of_truth.status=pass`.
3. Formal invariants remain green.

## BL-034 Incident Contract

This section is the enforced contract for incident + rollback drill coverage.

Required incident classes and deterministic first actions:

1. `routing_degraded`
   - Run: `node client/runtime/systems/routing/model_router.ts doctor --risk=low --complexity=low --intent=ops_triage --task="routing incident diagnosis"`
2. `schema_drift`
   - Run: `node client/runtime/systems/security/schema_contract_check.ts run --strict=1`
3. `sensory_starvation`
   - Run: `node client/cognition/habits/scripts/external_eyes.ts preflight --strict=0`
4. `autonomy_stall`
   - Run: `node client/runtime/systems/autonomy/ops_dashboard.ts status`

Rollback drill contract (weekly):

1. Engage containment:
   - `node client/runtime/systems/security/emergency_stop.ts engage --scope=autonomy --approval-note="rollback drill"`
2. Execute rollback target check:
   - `node client/runtime/systems/autonomy/improvement_controller.ts evaluate --force=1 --auto-revert=1`
3. Release containment:
   - `node client/runtime/systems/security/emergency_stop.ts release --approval-note="rollback drill complete"`
4. Record verification artifact:
   - Write receipt under `state/ops/evidence/rollback_drill_<YYYY-MM-DD>.md`
