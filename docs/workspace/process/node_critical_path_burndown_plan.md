# Node Critical-Path Burn-down Plan

Status: active migration plan for operator-critical Node paths.

## Purpose

Define owner-assigned, date-bound migration commitments for operator-critical command paths while enforcing TypeScript confinement to non-authoritative layers.

## Canonical Sources

- Burn-down policy: `client/runtime/config/node_critical_path_burndown_plan.json`
- Inventory/guard gate: `tests/tooling/scripts/ci/node_critical_path_inventory.ts`
- Gate id: `ops:node-critical-path:inventory`

## Required Domains

The plan must cover all of:

- `release`
- `repair`
- `topology_truth`
- `recovery`
- `status`

Each required domain must have at least one priority-1 lane with:

- explicit owner
- explicit target date
- explicit target classification
- explicit current runtime path
- explicit Rust replacement command or module

## Confinement Rule

Any Node TypeScript critical path must stay inside allowed non-authoritative Shell/flex/test surfaces:

- `tests/tooling/scripts/`
- `client/runtime/systems/ui/`
- `client/runtime/systems/extensions/`
- `client/runtime/systems/marketplace/`

Anything outside the allowlist is a fail-closed gate violation.

## Operator-Critical Node Exemptions

Operator-critical lanes may not depend on Node unless they have an explicit, bounded exemption in the burn-down plan.

Every exemption must include:

- `id`
- `owner`
- `reason`
- `expires_at`
- `status`

Active exemptions fail closed when they expire. Missing or malformed exemptions fail closed immediately.

## Migration Semantics

- `target_classification=rust_native` means lane is expected to migrate off Node by the target date.
- `target_runtime_path` identifies the current file or entrypoint being burned down.
- `target_rust_replacement` identifies the Rust replacement command or module that absorbs the lane.
- If target date is passed and lane is still not at target classification, gate fails.
- `target_classification=node_typescript` is allowed only for governance/flexibility lanes that remain explicitly non-authoritative and confined.
- Operator-critical domains are explicitly tracked (`release`, `repair`, `topology_truth`, `recovery`, `status`) and priority-1 lanes in those domains must target `rust_native`.
- Priority-1 operator-critical lane target dates are capped by `operator_critical_priority_cutoff_date`; dates beyond cutoff fail closed.

## Ordered Migration Queue

`client/runtime/config/node_critical_path_burndown_plan.json` defines `ordered_migration_queue`.

The queue is mandatory and CI fail-closes on:

- missing queue
- duplicate queue IDs
- queue IDs not present in declared lane set
- missing priority-1 operator-critical lanes in the queue

This keeps migration execution deterministic and prevents silent priority drift.

## Release Integration

Release evidence explicitly executes and captures this gate before proof-pack assembly in `.github/workflows/release.yml`.
