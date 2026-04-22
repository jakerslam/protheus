# Shell Canonicalization Notes (Internal)

Status: canonical-only
Date: 2026-04-22

## Scope

- Canonical presentation-layer term is `Shell`.
- Repository implementation path remains `client/**` until an explicit path migration program is approved.

## Rules

1. Architecture and operator docs must use `Shell`.
2. Path references stay stable unless an explicit migration task is approved.
3. Keep boundary language aligned with the architecture axiom:
   - Kernel decides truth and permission.
   - Orchestration decides flow and sequencing.
   - Shell decides rendering/input/UX collection.

## Historical Evidence Policy

- Historical ledgers and archived snapshots may preserve legacy wording as immutable evidence.
- New or updated non-historical docs must remain canonical-only.

## Tracking Guard (CI)

- Command: `npm run -s ops:shell-transition:tracker`
- Artifact: `core/local/artifacts/shell_transition_tracker_current.json`
- Report: `local/workspace/reports/SHELL_TRANSITION_TRACKER_CURRENT.md`
- Canonical map: `client/runtime/config/shell_transition_alias_map.json`
