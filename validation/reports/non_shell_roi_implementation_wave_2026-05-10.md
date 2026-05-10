# Non-Shell / Non-Orchestration ROI Implementation Wave

Generated: 2026-05-10

Scope: Kernel, Gateways, Validation, Observability, installers, release, command surface, proof packs, and repo hygiene. Shell and Orchestration implementation paths were intentionally excluded.

## Implemented Enforcement / Closure

1. Proof-pack artifact size policy and guard.
2. Compacted the two oversized checked-in Sentinel proof-pack reports into bounded reference indexes.
3. Sentinel full-run stage timing contract and source instrumentation.
4. Bounded Sentinel maintenance auto path revalidated through the fresh-evidence guard.
5. Command registry metadata policy and guard.
6. Command registry entries enriched with domain, work gate, description, and non-unclassified owner values.
7. CI workflow tier policy and tier report guard.
8. Universal trace completeness warning guard for non-Shell/non-Orchestration artifacts.
9. Gateway adapter invariant policy and guard.
10. Windows locked-down install replay policy and guard.
11. Release/version convergence policy and guard.
12. Rust crate support-status manifest and guard for 58 crates.

## Guard Results

- `proof_pack_artifact_size_guard`: pass
- `command_registry_metadata_guard`: pass, 1140 entries covered
- `ci_workflow_tier_guard`: pass, 52 workflows classified
- `rust_crate_support_status_guard`: pass, 58 crates classified
- `gateway_adapter_invariants_guard`: pass
- `windows_locked_down_install_replay_guard`: pass
- `release_version_convergence_guard`: pass
- `sentinel_stage_timing_policy_guard`: pass
- `non_shell_trace_completeness_guard`: pass in warning mode; 661 artifacts scanned
- `ops:kernel-sentinel:auto -- --max-runtime-ms=30000`: pass
- `ops:ksent:fresh-evidence:guard`: pass

## Remaining Follow-Through Batches

- Convert command registry metadata from inferred descriptions to curated operator-grade descriptions for top 100 commands.
- Move command surface from package script sprawl toward command-runner-first usage.
- Wire actual trace emission into Kernel, Gateway, Validation, and Sentinel producers rather than warning-only trace completeness.
- Split installer internals into executable modules instead of contract-only modules.
- Add actual Windows locked-down installer replay in CI using a Windows runner.
- Use CI tier report to reduce required check noise and move advisory/nightly checks out of normal push pressure.
- Promote production-candidate Rust crates with targeted validation evidence and demote/retire lab crates that are not useful.
- Add full Sentinel dream/release run timing trend analysis once full runs complete under dream cadence.

## Continue wave: enforcement and trace propagation

Additional non-Shell/non-orchestration follow-through completed:

- Upgraded CI workflow tiering from classification output to manifest enforcement.
  - Policy: `validation/conformance/contracts/ci_workflow_tier_policy.json`
  - Manifest: `validation/conformance/contracts/ci_workflow_tier_manifest.json`
  - Guard: `tests/tooling/scripts/ci/ci_workflow_tier_guard.ts`
  - Generator: `tests/tooling/scripts/ci/ci_workflow_tier_manifest_generate.ts`
- Upgraded command runner into a metadata-aware agent navigation surface.
  - Runner: `tools/commands/command_runner.ts`
  - Policy: `validation/conformance/contracts/command_runner_first_policy.json`
  - Guard: `tests/tooling/scripts/ci/command_runner_first_guard.ts`
- Added executable installer module seams so future installer shrinkage has a safe target.
  - Modules: `install/modules/windows_wrappers.ps1`, `install/modules/completion_card.ps1`, `install/modules/bootstrap_common.sh`
  - Policy: `validation/conformance/contracts/installer_module_policy.json`
  - Guard: `tests/tooling/scripts/ci/installer_module_guard.ts`
- Added Gateway trace propagation at the adapter boundary.
  - Preserves or mints `x-infring-trace-id`.
  - Sets response trace header.
  - Forwards trace ID through proxied API and websocket upgrade requests.
  - Policy: `observability/traces/gateway_trace_propagation_policy.json`
  - Guard: `tests/tooling/scripts/ci/gateway_trace_propagation_guard.ts`

Validation run:

- `ci_workflow_tier_manifest_generate.ts`
- `ci_workflow_tier_guard.ts`
- `command_runner_first_guard.ts`
- `installer_module_guard.ts`
- `gateway_trace_propagation_guard.ts`
- TypeScript transpile syntax check for `adapters/runtime/infring_dashboard.ts`

Remaining deeper follow-through:

- Convert CI tier manifest into branch-protection/release-gate behavior rather than only guard output.
- Curate command metadata beyond inferred values.
- Promote/demote Rust crate support status based on actual validation evidence.
- Add Sentinel dream/release timing trend analysis over multiple runs.
- Add deeper Gateway idempotence runtime replay.

## Continue wave: remaining five follow-through items

Completed the remaining non-Shell/non-orchestration follow-through items:

1. CI tier manifest release enforcement
   - Added `.github/workflows/ci-tier-enforcement.yml` so CI tier drift is checked in GitHub Actions.
   - Added `validation/release_gates/policies/ci_tier_release_enforcement_policy.json`.
   - Added `tests/tooling/scripts/ci/ci_tier_release_enforcement_guard.ts`.

2. Curated command metadata
   - Added `validation/conformance/contracts/command_metadata_curated_overrides.json` with 20 high-value command overrides.
   - Updated `tools/commands/command_registry.json` for Sentinel, Gateway, installer, release, security, memory, and command-runner commands.
   - Added `tests/tooling/scripts/ci/command_metadata_curated_guard.ts`.

3. Rust crate support evidence
   - Added `validation/conformance/contracts/rust_crate_support_evidence_policy.json`.
   - Added `tests/tooling/scripts/ci/rust_crate_support_evidence_report.ts`.
   - Added `tests/tooling/scripts/ci/rust_crate_support_evidence_guard.ts`.
   - Generated `validation/reports/rust_crate_support_evidence_report_2026-05-10.json` for 58 crates.

4. Sentinel timing trend analysis
   - Added `observability/sentinel/sentinel_timing_trend_policy.json`.
   - Added `tests/tooling/scripts/ci/sentinel_timing_trend_report.ts`.
   - Added `tests/tooling/scripts/ci/sentinel_timing_trend_guard.ts`.
   - Generated `observability/reports/sentinel_timing_trend_report_2026-05-10.json`.
   - Current status is `insufficient_samples`, which is expected until dream/release full runs accumulate timing samples.

5. Gateway idempotence replay
   - Added replay fixtures under `validation/regression/fixtures/gateway_idempotence/`.
   - Added `tests/tooling/scripts/ci/gateway_idempotence_replay_guard.ts`.
   - Scenarios cover start-while-active, restart preserving port 4173, and read-only status behavior.

Validation run:

- `ci_tier_release_enforcement_guard.ts`
- `command_metadata_curated_guard.ts`
- `rust_crate_support_evidence_report.ts`
- `rust_crate_support_evidence_guard.ts`
- `sentinel_timing_trend_report.ts`
- `sentinel_timing_trend_guard.ts`
- `gateway_idempotence_replay_guard.ts`

All completed guards passed.
