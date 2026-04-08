# Backlog Registry

`client/runtime/systems/ops/backlog_registry.ts` (runtime: `client/runtime/systems/ops/backlog_registry.ts`) provides a canonical machine-readable backlog source and generated markdown views.

## Commands

```bash
node client/runtime/systems/ops/backlog_registry.ts sync
node client/runtime/systems/ops/backlog_registry.ts check --strict=1
node client/runtime/systems/ops/backlog_registry.ts status
```

## Policy

Policy file: `client/runtime/config/backlog_registry_policy.json`

Outputs:
- Canonical registry: `client/runtime/config/backlog_registry.json`
- Active view: `docs/client/backlog_views/active.md`
- Archive view: `docs/client/backlog_views/archive.md`
- Receipts: `state/ops/backlog_registry/latest.json`, `state/ops/backlog_registry/receipts.jsonl`

`check --strict=1` fails when generated artifacts drift from the canonical backlog markdown.

## Conduit Rebuild Chain

Conduit implementation/recovery is explicitly tracked as dependency-linked backlog items:

- `V6-CONDUIT-001` through `V6-CONDUIT-008`

These rows are the authoritative replay chain for rebuilding conduit requirements after regression.
