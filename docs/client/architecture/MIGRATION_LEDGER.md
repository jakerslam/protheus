# Migration Ledger

Tracks temporary compatibility surfaces that must be removed after core-authoritative cutovers.

## Active Compatibility Surfaces

| Surface | Current Role | Target Authority | Removal Trigger |
|---|---|---|---|
| `client/runtime/systems/spine/spine.ts` | Dev-mode compatibility shell | `core/layer0` spine runtime + conduit | `spine.ts` no longer referenced by package/bin/ops scripts |
| `client/runtime/systems/spine/spine_safe_launcher.ts` | Legacy launcher shell | Rust spine via conduit bridge | `spine_safe_launcher` callers migrated to direct conduit lane |
| `client/runtime/systems/spine/heartbeat_trigger.ts` | Legacy trigger shell | Rust heartbeat policy | heartbeat scheduler references only conduit-managed runtime |
| `client/runtime/lib/legacy_retired_lane_bridge.ts` | Legacy conduit fallback shim | `client/runtime/lib/spine_conduit_bridge.ts` and domain bridges | no production path imports legacy bridge |
| `client/runtime/systems/ops/infringd.ts` local fallback lane | Operational compatibility path | conduit-only runtime control | `--allow-legacy-fallback` path retired |
| `client/runtime/systems/adaptive/core/*` TS primitives | Temporary adaptation bootstrap authority | `core/layer2` adaptation primitives (REQ-19 set) | `V6-ADAPT-CORE-001` complete |
| legacy runtime roots (`state/`, `.clawhub/`, `.private-lenses/`, `client/logs/`) | Legacy mutable artifact paths | `client/runtime/local/*`, `core/local/*` partitions | `LOCAL-PARTITION-001` fully migrated and root-level duplicates removed |

## Backlog Anchors

- `V6-ADAPT-CORE-001`: move adaptation authority to `core/layer2`; keep client as conduit shell.
- `LOCAL-PARTITION-001`: complete runtime write migration to `client/runtime/local` and `core/local`.
- `PLANES-METAKERNEL-001`: keep architecture docs, schemas, and runtime contracts aligned with `planes/{safety,cognition,substrate}`.

## Policy Rules

1. Compatibility surfaces cannot gain new policy authority.
2. Any compatibility lane change must include a deprecation exit criterion.
3. Kernel/client transfer must remain conduit + scrambler only.
4. Remove entries from this ledger only in the same change that removes the compatibility path.
