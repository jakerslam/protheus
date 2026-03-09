# Root Ownership Map

Defines ownership intent for repository-root entries after the core/client split.

## Source Code Directories

- `core/`: core authority implementation (`layer_minus_one`, `layer0`, `layer1`, `layer2`, `layer3`).
- `client/`: surface implementation (TS/JS/Python/Shell/PowerShell + tests).
- `planes/`: architecture contracts (safety/cognition/substrate) and schemas.
- `examples/apps/`: optional top-of-client application/tool workspaces (default local-first, explicitly allowlisted tools may be tracked).

## Infrastructure/Metadata Directories

- `.github/`: CI workflows and branch policy.
- `.githooks/`: local hook helpers.
- `dist/`: generated build output.
- `target/`: Rust build artifacts.
- `node_modules/`: npm dependency cache.
- `client/runtime/local/workspaces/`: relocated local-only sidecars/scratch workspaces (ignored).

## Root File Classes

- Governance + narrative: `docs/workspace/SRS.md`, `docs/workspace/TODO.md`, `docs/workspace/UPGRADE_BACKLOG.md`, `docs/workspace/AGENTS.md`, `docs/workspace/AGENT-CONSTITUTION.md`.
- Product/repo metadata: `README.md`, `LICENSE`, `docs/workspace/CONTRIBUTING.md`, `SECURITY.md`, `docs/workspace/CHANGELOG.md`.
- Build and package manifests: `Cargo.toml`, `Cargo.lock`, `package.json`, `package-lock.json`.
- Runtime/infra bootstrap: `Dockerfile`, `docker-compose.yml`, `install.sh`, `install.ps1`, `tsconfig*.json`, `vitest.config.ts`.
- Bootstrap identity/memory docs (workspace references under docs): `docs/workspace/MEMORY.md`, `docs/workspace/MEMORY_INDEX.md`, `docs/workspace/TAGS_INDEX.md`, `docs/workspace/SOUL.md`, `docs/workspace/USER.md`, `docs/workspace/HEARTBEAT.md`, `docs/workspace/IDENTITY.md`, `docs/workspace/TOOLS.md`.

## Root Exception Rationale

- The bootstrap identity/memory docs are tracked under `docs/workspace/` and resolved by runtime/config policy paths.
- These files are explicitly allowlisted in `client/runtime/config/root_surface_contract.json` and validated by `root_surface_contract` checks.
- This is a policy exception, not a loophole: new runtime data must still live under `client/runtime/local/*` or `core/local/*`.

## Guarding Rules

1. New source code must land under `core/` or `client/` only.
2. Legacy root runtime folders (`adaptive`, `memory`, `habits`, `logs`, `patches`, `reports`, `research`, `secrets`, `state`, `.clawhub`, `.private-lenses`) are disallowed.
3. Root sidecar/scratch dirs (`agent-holo-viz`, `pqts`, `projects`, `rohan-*`, `tmp`) are disallowed and must live under `client/runtime/local/workspaces/`.
4. Runtime mutable data belongs in `client/runtime/local/*` and `core/local/*`.
5. Root allowances are enforced by `ops:root-surface:check` and `ops:source-runtime:check`.
