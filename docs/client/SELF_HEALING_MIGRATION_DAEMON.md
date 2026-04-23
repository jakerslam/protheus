# Self-Healing Migration Daemon

`V4-MIGR-004` continuously checks for legacy repository posture and guides consent-gated migration.

## Responsibilities

- Detect legacy/missing remotes at startup or on demand.
- Emit signed detector receipts and migration suggestions.
- Feed suggestions into self-audit suggestion stream.
- Support one-click upgrade path with explicit consent token.

## Commands

```bash
# Detect only
node client/runtime/systems/migration/self_healing_migration_daemon.ts scan --workspace=. --strict=0

# Consent-gated upgrade (invokes core migration bridge)
node client/runtime/systems/migration/self_healing_migration_daemon.ts upgrade \
  --workspace=. \
  --to=acme/infring-core \
  --workspace-target=../infring-core \
  --consent-token=MIGR-CONSENT-20260303
```

Receipts live under `state/migration/self_healing/` and suggestions are mirrored to `state/self_audit/illusion_integrity_suggestions.jsonl`.
