# Scale Readiness Program

`V4-SCALE-001` through `V4-SCALE-010` are implemented by `client/runtime/systems/ops/scale_readiness_program.ts`.

## Run

```bash
node client/runtime/systems/ops/scale_readiness_program.ts run-all --apply=1 --strict=1
node client/runtime/systems/ops/scale_readiness_program.ts status
```

This lane emits reproducible receipts and writes durable contracts under `client/runtime/config/scale_readiness/`.
