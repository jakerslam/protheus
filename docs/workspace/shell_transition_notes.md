# Shell Transition Notes (Internal)

Status: active docs-first transition
Date: 2026-04-20

## Goal

Use `Shell` as the canonical presentation-layer term across architecture and operator docs, while preserving runtime compatibility with existing `client/**` paths and command IDs.

## Canonical Mapping

| Canonical Term | Compatibility Alias | Current Path/ID Surface |
| --- | --- | --- |
| `Shell` | `Client` | `client/**`, `ops:client-*` guard IDs, `docs/client/**` links |

## Immediate Rules

1. New or updated architecture-facing docs should say `Shell` first.
2. Any path references remain unchanged unless an explicit migration task is approved.
3. Where ambiguity is possible, use: `Shell (compat alias: Client, repo path client/**)`.
4. Keep policy language consistent with the boundary axiom:
   - Kernel decides truth and permission.
   - Orchestration decides flow and sequencing.
   - Shell decides rendering/input/UX collection.

## Compatibility Contract (Do Not Break)

- Keep these stable during this phase:
  - `client/runtime/**` paths
  - `docs/client/**` documentation paths
  - `ops:client-*` command IDs and CI gate names
- Any alias removal must be scheduled in a release policy with:
  - target version/date
  - rollback plan
  - migration evidence

Historical-log note:
- Backlog/history ledgers (for example `docs/workspace/SRS.md`) may retain older `Client` wording inside historical rows; this is treated as historical evidence text, not current canonical terminology.

## Follow-On Migration Backlog (when approved)

1. Expand `ops:shell-*` aliases across remaining `ops:client-*` gates (naming guard aliases now in place).
2. Add a shell transition alias manifest for command/tooling IDs.
3. Introduce docs link aliases for `docs/shell/**` while preserving `docs/client/**`.
4. Plan path migration only after guard aliasing and release policy are complete.

## Tracking Guard (CI)

- Command: `npm run -s ops:shell-transition:tracker`
- Artifact: `core/local/artifacts/shell_transition_tracker_current.json`
- Report: `local/workspace/reports/SHELL_TRANSITION_TRACKER_CURRENT.md`
