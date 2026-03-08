# Public Operator Profile

This document defines the external, operator-facing surface for OpenClaw.

## What OpenClaw Is

OpenClaw is a local automation and orchestration runtime for macOS/Linux with typed control-plane lanes, policy contracts, and deterministic receipts.

## Public-First Entry Points

- `README.md` for overview and quick start
- `docs/client/README.md` for navigation
- `docs/client/OPERATOR_RUNBOOK.md` for incident/operations procedures
- `CONTRIBUTING.md`, `SECURITY.md`, and `LICENSE` for contribution and policy posture

## Internal Artifact Handling

Persona- and memory-heavy root artifacts are treated as internal compatibility surfaces and mirrored under `docs/client/internal/persona/`.

The canonical internal aliases are:

- `AGENT-CONSTITUTION.md` -> `docs/client/internal/persona/AGENT-CONSTITUTION.md`
- `IDENTITY.md` -> `docs/client/internal/persona/IDENTITY.md`
- `SOUL.md` -> `docs/client/internal/persona/SOUL.md`
- `USER.md` -> `docs/client/internal/persona/USER.md`
- `MEMORY.md` -> `docs/client/internal/persona/MEMORY.md`
- `codex.helix` -> `docs/client/internal/persona/CODEX_HELIX.md`

## Regression Gates

Public-surface regressions are enforced by:

- `npm run -s ops:docs-surface:check`
- `npm run -s ops:root-surface:check`
- `npm run -s ops:path-contract:check`
