# RUNBOOK-007: Pure/Tiny Capability Restore

## Purpose

Restore and verify the `InfRing (pure)` and `InfRing (tiny-max)` intelligence surfaces if a regression removes parity or breaks hardware-aware capability shedding.

This runbook is the authoritative recovery path for:
- pure-mode Rust-core orchestration/swarm access
- tiny-max capability shedding by hardware class
- fail-closed behavior on constrained hardware profiles

## Scope

This runbook covers the following surfaces:

- Kernel authority:
  - `core/layer0/ops/src/infringd.rs`
  - `core/layer0/ops/tests/v9_pure_capability_profile_cli.rs`
- Thin pure client passthrough:
  - `client/pure-workspace/src/main.rs`
- Operator command discoverability:
  - `client/runtime/systems/ops/infring_command_list.js`
- Documentation contract:
  - `README.md` mode matrix section

## Required Invariants

The system is healthy only if all invariants below hold:

1. `infringd` exposes:
   - `orchestration`
   - `swarm-runtime`
   - `capability-profile`
2. `infring-pure-workspace` forwards:
   - `orchestration`
   - `swarm-runtime`
   - `capability-profile`
   to `infringd` (no Node/TS intelligence fallback).
3. Hardware-class shedding is enforced in daemon (not advisory):
   - `mcu` blocks heavy orchestration ops (for example `coordinator.run`)
   - `mcu` blocks `research fetch`
   - `mcu` enforces `max_swarm_depth=1`
   - persistent swarm is disabled for constrained classes
4. `think` clamps memory retrieval by capability profile (`effective_memory_limit`).
5. All behavior is fail-closed (returns deterministic error receipts when blocked).

## Regression Symptoms

Common signs of regression:

- `infring orchestration ...` fails in pure/tiny with `unknown_command`
- pure mode cannot access swarm runtime
- `capability-profile` missing or returns static/non-sensed data
- `--hardware-class=mcu` still allows `coordinator.run`
- `--hardware-class=mcu` still allows `research fetch`
- swarm spawn on mcu accepts `--max-depth > 1`

## Restore Procedure

1. Validate file presence and command surfaces.
2. Reapply missing daemon routes and profile validation gates.
3. Reapply thin-client passthrough routes in `client/pure-workspace`.
4. Re-run deterministic test/CLI proofs below.
5. Run security + churn gates before merge.

## Verification Commands (Must Pass)

### 1) Kernel unit/regression tests

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --bin infringd -- --nocapture
```

Expected:
- tests for capability shedding and `think` clamping pass.

### 2) CLI integration tests (pure/tiny restore contract)

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --test v9_pure_capability_profile_cli -- --nocapture
```

Expected:
- `capability_profile_reports_mcu_shedding`
- `microcontroller_profile_blocks_heavy_orchestration_op`
- `microcontroller_profile_blocks_swarm_depth_overflow`
all pass.

### 3) Pure workspace tests

```bash
cargo test --manifest-path client/pure-workspace/Cargo.toml -- --nocapture
```

Expected:
- pure and tiny profile targets pass.

### 4) Runnable CLI evidence

```bash
cargo run --quiet --manifest-path core/layer0/ops/Cargo.toml --bin infringd -- capability-profile --hardware-class=mcu --tiny-max=1 --memory-mb=256 --cpu-cores=1
```

Expected payload includes:
- `type=infringd_capability_profile`
- `profile.hardware_class=mcu`
- `profile.capabilities.research_fetch=false`
- shed capabilities include `swarm.max_depth>1`

```bash
cargo run --quiet --manifest-path core/layer0/ops/Cargo.toml --bin infringd -- research fetch --url=https://example.com --hardware-class=mcu --tiny-max=1
```

Expected:
- fail-closed response with `error=hardware_profile_blocks_research_fetch`.

```bash
cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infringd
PATH=\"$(pwd)/target/debug:$PATH\" cargo run --quiet --manifest-path client/pure-workspace/Cargo.toml -- orchestration invoke --op=coordinator.partition --payload-json='{\"items\":[\"a\",\"b\",\"c\"],\"agent_count\":2}'
PATH=\"$(pwd)/target/debug:$PATH\" cargo run --quiet --manifest-path client/pure-workspace/Cargo.toml -- swarm-runtime status
```

Expected:
- orchestration partition result emitted
- swarm runtime status receipt emitted

### 5) Sovereignty/security gate

```bash
npm run -s test:security:truth-gate
```

Expected:
- `ok: true`

### 6) Churn/commit hygiene gate

```bash
npm run -s ops:churn:guard
```

Expected:
- no unresolved move/untracked churn before merge.

## Recovery Decision Table

- Missing command routes in `infringd`:
  - restore daemon `match` routes and usage text.
- Missing pure passthrough:
  - restore `client/pure-workspace/src/main.rs` forwarding branches.
- Capability shedding bypassed:
  - restore hardware-profile validators and fail-closed errors in daemon.
- Tests missing:
  - restore `core/layer0/ops/tests/v9_pure_capability_profile_cli.rs`.

## Notes

- `pure` and `tiny-max` are layered capability profiles on one Rust core, not independent feature forks.
- Shedding policy is expected behavior on constrained hardware and should be explicit in receipts.
