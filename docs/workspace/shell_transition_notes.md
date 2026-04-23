# Shell Canonicalization Notes (Internal)

Status: compatibility-bridge-active
Date: 2026-04-23

## Scope

- Canonical presentation-layer term is `Shell`.
- Repository implementation path remains `client/**` until an explicit path migration program is approved.
- Temporary Shell/Client compatibility bridges remain active only until `2026-07-15`.

## Goal

Retire temporary Shell/Client compatibility bridges on or before `2026-07-15` while keeping operator-facing naming canonical-first (`Shell`).

## Rules

1. Architecture and operator docs must use `Shell`.
2. Path references stay stable unless an explicit migration task is approved.
3. Keep boundary language aligned with the architecture axiom:
   - Kernel decides truth and permission.
   - Orchestration decides flow and sequencing.
   - Shell decides rendering/input/UX collection.
4. Any Shell compatibility bridge must include owner + explicit removal deadline.

## Historical Evidence Policy

- Historical ledgers and archived snapshots may preserve legacy wording as immutable evidence.
- New or updated non-historical docs must remain canonical-only.

## Tracking Guard (CI)

- Command: `npm run -s ops:shell-transition:tracker`
- Artifact: `core/local/artifacts/shell_transition_tracker_current.json`
- Report: `local/workspace/reports/SHELL_TRANSITION_TRACKER_CURRENT.md`
- Canonical map: `client/runtime/config/shell_transition_alias_map.json`
- Compatibility bridges: `client/runtime/config/shell_transition_compatibility_bridges.json`
