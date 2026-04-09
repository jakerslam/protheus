# Rust Spine Microkernel

`V3-RACE-034` promotes explicit Rust-spine control-plane readiness for:

- `guard`
- `spawn_broker`
- `model_router`
- `origin_lock`
- `fractal_orchestrator`

Lane entrypoint: `client/runtime/systems/ops/rust_spine_microkernel.ts`

## Commands

```bash
node client/runtime/systems/ops/rust_spine_microkernel.ts parity --apply=1
node client/runtime/systems/ops/rust_spine_microkernel.ts benchmark --apply=1
node client/runtime/systems/ops/rust_spine_microkernel.ts cutover --apply=1
node client/runtime/systems/ops/rust_spine_microkernel.ts route --component=guard
node client/runtime/systems/ops/rust_spine_microkernel.ts rollback --reason=manual --apply=1
node client/runtime/systems/ops/rust_spine_microkernel.ts status
```

Cutover requires parity streak + SLO pass, and rollback forces emergency JS routing.
