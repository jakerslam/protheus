# Rust Workspace Quality Gate

`V4-RUST-003` adds a unified workspace + quality gate for Rust lanes.

## Required Artifacts

- Root `Cargo.toml` workspace manifest
- Root `rust-toolchain.toml`
- Generated type references:
  - `docs/client/generated/TS_LANE_TYPE_REFERENCE.md`
  - `docs/client/generated/RUST_LANE_TYPE_REFERENCE.md`

## Commands

```bash
node client/runtime/systems/ops/rust_workspace_quality_gate.js run --strict=1 --apply=1
node client/runtime/systems/ops/rust_workspace_quality_gate.js status
```

By default, the gate enforces manifest/toolchain/docs presence + `cargo metadata` validity.
Optional strict checks (`fmt`, `clippy`, `test`) are policy-gated.
