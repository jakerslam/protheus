# Anti-Entropy Next Items Wave - 2026-05-10

Scope: non-Shell, non-orchestration implementation focused on agent usefulness and system anti-decay.

## Completed

1. Safe commit/worktree safety report
   - Added `tools/git/safe_commit_report.ts`.
   - Added `validation/conformance/contracts/safe_commit_workspace_policy.json`.
   - Added `tests/tooling/scripts/ci/safe_commit_workspace_guard.ts`.
   - Generated `validation/reports/safe_commit_workspace_report_2026-05-10.json`.

2. Command runner default simplification
   - Updated `tools/commands/command_runner.ts` so default `cmd list` hides compatibility aliases.
   - Full command surface remains available with `--include-compat=1`.
   - Current default list: 20 canonical/curated entries.
   - Full compatibility list: 1140 entries.

3. CI required-gate reduction plan
   - Added `validation/release_gates/policies/ci_required_gate_reduction_policy.json`.
   - Added `tests/tooling/scripts/ci/ci_required_gate_reduction_report.ts`.
   - Added `tests/tooling/scripts/ci/ci_required_gate_reduction_guard.ts`.
   - Generated `validation/reports/ci_required_gate_reduction_plan_2026-05-10.json`.
   - Current required count: 48.
   - Target required max: 18.
   - Recommended demotion candidates: 38.

4. Sentinel fresh-evidence trace completion
   - Updated `tests/tooling/scripts/ci/kernel_sentinel_fresh_evidence_guard.ts` to emit `trace_id`, `span_id`, and `source_domain`.

5. Sentinel full-run timeout classification
   - Attempted a full dream Sentinel run with a 120s stall guard.
   - Result: bounded timeout diagnostic, not a hang.
   - Added `observability/sentinel/sentinel_full_run_timeout_policy.json`.
   - Added `tests/tooling/scripts/ci/sentinel_full_run_timeout_report.ts`.
   - Added `tests/tooling/scripts/ci/sentinel_full_run_timeout_guard.ts`.
   - Generated `observability/reports/sentinel_full_run_timeout_report_2026-05-10.json`.
   - Finding: full Sentinel dream/release self-study currently exceeds bounded run budget and should be split into resumable stages or given an explicit larger dream budget.

6. Gateway live status replay
   - Added `validation/regression/fixtures/gateway_idempotence/gateway_status_live_replay_policy.json`.
   - Added `tests/tooling/scripts/ci/gateway_status_live_replay.ts`.
   - Added `tests/tooling/scripts/ci/gateway_status_live_replay_guard.ts`.
   - Generated `validation/reports/gateway_status_live_replay_2026-05-10.json`.
   - Finding: `gateway:status` is blocked by `security_gate_blocked:embedded_checker_failed:embedded_security_checker_not_linked_use_cargo; cargo_fallback_disabled`.

## Validation

Passed:

- `safe_commit_workspace_guard.ts`
- `command_runner_first_guard.ts`
- `ci_required_gate_reduction_guard.ts`
- `kernel_sentinel_fresh_evidence_guard.ts --strict=1`
- `sentinel_full_run_timeout_guard.ts`
- `gateway_status_live_replay_guard.ts`

## Open findings created by this wave

1. Full Sentinel dream/release run is too heavy for 120s bounded execution.
2. Gateway status is blocked before read-only diagnostics because embedded security checker linkage is missing and cargo fallback is disabled.
3. Normal commit flow is unsafe in the current local workspace due to unmerged path, shadow delete/untracked pairs, and large dirty surface.
4. CI required-gate surface is too large for fast iteration.
