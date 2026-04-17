# Backend File-Size and Cohesion Burn-Down Plan

Generated: 2026-04-15

## Scope

Kick off backend debt reduction for release blockers from:

- `core/local/artifacts/rust_core_file_size_gate_current.json`
- `core/local/artifacts/module_cohesion_audit_current.json`

This plan is the first wave (start), not the full migration.

## Wave 1 (Immediate)

1. Split `core/layer0/ops/src/assimilate_kernel_support.rs` into `assimilate_kernel_support_parts/` by command lane.
2. Split `core/layer0/ops/src/dashboard_terminal_broker.rs` into request parsing, routing, and receipts modules.
3. Split `core/layer2/tooling/src/tool_broker.rs` into capability probing, execution dispatch, and result normalization modules.
4. Keep behavior unchanged; add/retain parity tests per split.

## Wave 2 (High ROI)

1. Decompose `core/layer2/execution/src/autoscale_parts/400-run-autoscale-json.rs`.
2. Decompose `core/layer2/execution/src/autoscale_parts/410-critical-pressure-scales-up.rs`.
3. Decompose `core/layer2/execution/src/inversion_parts/220-normalize-impact-matches-expected-set.rs`.

## Guardrails

1. No authority migration out of `core/**`.
2. No behavior changes unless explicitly requested.
3. For each split, run targeted lane tests plus one regression command.

## Exit signal for this kickoff

1. Wave 1 file decomposition merged.
2. Re-run `ops:rust-core-file-size:gate` and `audit:module-cohesion`.
3. Document remaining top offenders for Wave 2.
