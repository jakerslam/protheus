# REQ-26 — TCB Systems Rust Authority Cutover

Status: in_progress  
Owner: Protheus Kernel  
Updated: 2026-03-06

## Objective

Enforce Rust as the runtime source of truth for TCB system domains while preserving TS as thin surface code only.

TCB targets:

- `client/runtime/systems/security/`
- `client/runtime/systems/ops/`
- `client/runtime/systems/memory/`
- `client/runtime/systems/sensory/`
- `client/runtime/systems/autonomy/`
- `client/runtime/systems/assimilation/`

TS surface-only allowlist:

- `client/runtime/systems/ui/`
- `client/runtime/systems/marketplace/`
- `client/runtime/systems/extensions/`

## Acceptance Criteria

1. Kernel entrypoint launchers in the TCB paths dispatch to Rust binaries/crates.
2. Rust `protheus-ops` exposes native domains for any newly cutover controllers.
3. Policy gates encode TCB-required prefixes and TS surface allowlist.
4. `cargo test -p protheus-ops-core` and `cargo clippy -p protheus-ops-core --all-targets -- -D warnings` pass.
5. `formal:invariants:run` remains green.

## Phase-1 Deliverables (this batch)

- Restored Rust runtime dispatch shims for reverted launchers:
  - `client/runtime/systems/ops/protheusctl.ts`
  - `client/runtime/systems/ops/state_kernel.ts`
  - `client/runtime/systems/ops/autotest_controller.ts`
  - `client/runtime/systems/ops/autotest_doctor.ts`
  - `client/runtime/systems/autonomy/autonomy_controller.ts`
  - `client/runtime/systems/autonomy/health_status.ts`
  - `client/runtime/systems/autonomy/inversion_controller.ts`
  - `client/runtime/systems/autonomy/strategy_mode_governor.ts`
  - `client/runtime/systems/memory/idle_dream_cycle.ts`
  - `client/runtime/systems/memory/rust_memory_transition_lane.ts`
  - `client/runtime/systems/memory/memory_recall.ts` (mapped to `memory-cli` command semantics)
- Added Rust-native domains:
  - `assimilation-controller` (`core/layer0/ops/src/assimilation_controller.rs`)
  - `sensory-eyes-intake` (`core/layer0/ops/src/sensory_eyes_intake.rs`)
- Switched launcher entrypoints:
  - `client/runtime/systems/assimilation/assimilation_controller.ts`
  - `client/runtime/systems/sensory/eyes_intake.ts`
- Added shared bridge helper:
  - `client/runtime/lib/rust_lane_bridge.ts`
- Updated governance policy:
  - `client/runtime/config/rust_source_of_truth_policy.json`
  - `docs/workspace/codex_enforcer.md`

## Remaining Work

1. Port remaining non-wrapper TS logic in `security/sensory/assimilation/client/memory/autonomy` to Rust modules with behavior parity tests.
2. Retire legacy TS control-flow implementations after parity gates are green.
3. Extend policy/audit tooling to fail CI when new non-surface TS control logic appears under TCB prefixes.

## Phase-2 Deliverables (top-8 ops/security lanes)

- Added Rust domains in `core/layer0/ops`:
  - `execution_yield_recovery`
  - `protheus_control_plane`
  - `rust50_migration_program`
  - `venom_containment_layer`
  - `dynamic_burn_budget_oracle`
  - `backlog_registry`
  - `rust_enterprise_productivity_program`
  - `backlog_github_sync`
- Updated both TS and JS lane entrypoints for these domains to thin wrappers through `client/runtime/lib/rust_lane_bridge.ts`.
- Added CLI domains in `core/layer0/ops/src/main.rs` and module exports in `core/layer0/ops/src/lib.rs`.
