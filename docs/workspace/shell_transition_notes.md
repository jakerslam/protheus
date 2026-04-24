# Shell Canonicalization Notes (Internal)

Status: retired
Date: 2026-04-24

## Scope

- Canonical presentation-layer term is `Shell`.
- Repository implementation path remains `client/**` until an explicit path migration program is approved.
- Shell/Client compatibility bridges are retired. `client/**` remains an implementation path only, not a public layer name.

## Goal

Keep operator-facing naming canonical-first (`Shell`) with no active `Client` compatibility artifacts.

## Rules

1. Architecture and operator docs must use `Shell`.
2. Path references stay stable unless an explicit migration task is approved.
3. Keep boundary language aligned with the architecture axiom:
   - Kernel decides truth and permission.
   - Orchestration decides flow and sequencing.
   - Shell decides rendering/input/UX collection.
4. Any attempt to reintroduce a Shell compatibility bridge must include owner, explicit removal deadline, and break-glass justification.

## Historical Evidence Policy

- Historical ledgers and archived snapshots may preserve legacy wording as immutable evidence.
- New or updated non-historical docs must remain canonical-only.

## Tracking Guard (CI)

- Command: `npm run -s ops:shell-transition:tracker`
- Artifact: `core/local/artifacts/shell_transition_tracker_current.json`
- Report: `local/workspace/reports/SHELL_TRANSITION_TRACKER_CURRENT.md`
- Canonical map: `client/runtime/config/shell_transition_alias_map.json`
- Compatibility bridge ledger: `client/runtime/config/shell_transition_compatibility_bridges.json`
