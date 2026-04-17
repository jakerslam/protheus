# Runtime Proof Backlog TODO

This file tracks remaining execution from the runtime-proof hardening intake after the one-pass patch.

## Completed in this patch

- Added deterministic runtime-proof harness for:
  - 72h boundedness test
  - queue saturation test
  - conduit failure/recovery test
  - dashboard disconnect/reconnect test
  - adapter crash/restart test
- Added release gate policy thresholds in `tests/tooling/config/release_gates.yaml`.
- Added runtime-proof release gate with:
  - compact pass/fail markdown table
  - raw metrics JSON
  - strict fail mode for threshold regressions
- Wired runtime-proof verification into tooling registry + verify profile:
  - `ops:runtime-proof:verify`
  - `verify_profiles.runtime-proof`
  - release profile now includes runtime-proof verification
- Added operator scripts:
  - `ops:runtime-proof:harness`
  - `ops:runtime-proof:release-gate`
  - `ops:runtime-proof:verify`
  - `infring:verify:runtime-proof` (npm script bridge)
- Added Layer2 parity matrix + guard with contract test IDs:
  - `tests/tooling/config/layer2_lane_parity_manifest.json`
  - `tests/tooling/scripts/ci/layer2_lane_parity_guard.ts`
- Added deterministic Layer2 receipt replay with structural divergence reporting:
  - `tests/tooling/scripts/ci/layer2_receipt_replay.ts`
  - `tests/tooling/fixtures/layer2_receipt_bundle_golden.json`
- Added boundedness inspect command/reporting:
  - `tests/tooling/scripts/ci/runtime_boundedness_inspect.ts`
- Added trusted-core report + drift checks:
  - `tests/tooling/config/trusted_core_manifest.json`
  - `tests/tooling/scripts/ci/runtime_trusted_core_report.ts`
- Added release proof-pack assembly:
  - `tests/tooling/config/release_proof_pack_manifest.json`
  - `tests/tooling/scripts/ci/release_proof_pack_assemble.ts`
- Added public benchmark harness:
  - `benchmarks/public_harness/workloads.json`
  - `benchmarks/public_harness/run_public_harness.ts`
- Added CLI command routing for:
  - `infring verify layer2-parity`
  - `infring verify trusted-core`
  - `infring verify release-proof-pack`
  - `infring inspect boundedness`
  - `infring replay layer2`
- Added queue backpressure policy engine in core runtime:
  - `protheus-ops queue-sqlite-kernel backpressure-policy`
  - deterministic state/action mapping with defer/shed/quarantine policies and priority-aging multiplier

## Remaining TODO (next waves)

- Adapter contract-kit unification across first five adapters.
- Adapter chaos packs for startup/hang/flap/schema/oversize paths.
- Dashboard freshness model migration to receipt-derived authoritative state only.
- Conduit auto-heal lifecycle state machine receipts and policy gates.
- Rust authority migration for additional reliability-critical TS/Node orchestration hotspots.

## Observability + Tooling Contracts (new)

- Landed in this revision:
  - App-plane chat now emits capability discovery metadata in finalization receipts (`tool_execution_receipt_v1` contract + discovered tools + provider catalog).
  - Tool diagnostics now generate explicit per-call execution receipts with deterministic `call_id`, canonical status (`ok|error|blocked|not_found|low_signal|unknown`), and telemetry (`duration_ms`, `tokens_used`).
  - Silent/no-receipt tool outcomes now fail-closed into stable error code `web_tool_silent_failure`; missing tool paths classify as `web_tool_not_found`.
- Next wave TODO:
  - Propagate `trace_id` through dashboard tool turn loop, provider runtime adapters, and workflow synthesis boundaries (not just app-plane run receipts).
  - Add a span/export bridge for Rust `tracing` so tool call IDs and trace IDs can be joined in one searchable stream.
  - Add schema registry endpoint for agent capability discovery (tool name -> input/output contract -> auth/policy requirements) with deterministic snapshot receipts.
  - Add cross-lane contract tests proving every tool invocation path returns an execution receipt on success, timeout, policy block, and internal failure.
