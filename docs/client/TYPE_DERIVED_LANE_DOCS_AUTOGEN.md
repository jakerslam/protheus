# Type-Derived Lane Docs Autogeneration

`V3-RACE-230` keeps lane docs in sync with TS/Rust type surfaces and blocks stale documentation.

## Commands

```bash
node client/runtime/systems/ops/type_derived_lane_docs_autogen.ts generate --apply=1 --strict=1
node client/runtime/systems/ops/type_derived_lane_docs_autogen.ts verify --strict=1
node client/runtime/systems/ops/type_derived_lane_docs_autogen.ts rollback --apply=1
```

## Outputs

- `docs/client/generated/TS_LANE_TYPE_REFERENCE.md`
- `docs/client/generated/RUST_LANE_TYPE_REFERENCE.md`

The lane writes receipts and rollback snapshots under `state/ops/type_derived_lane_docs_autogen/`.
