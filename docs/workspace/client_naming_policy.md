# Shell Naming Policy (compat path: `client/**`)

## Goal

Keep shell surfaces readable at scale with stable, predictable naming and continuous tracking.

Current enforcement mode: **Yellow Flag (advisory)**.
- Naming violations are reported in artifacts and CI telemetry.
- They do not block merges by default.
- Strict blocking remains available as an explicit opt-in lane.

## Enforced Scope

The guard applies to:

- `client/runtime/lib/**`
- `client/runtime/systems/**`
- `client/cognition/orchestration/**`
- `client/cognition/shared/**`
- `client/cognition/habits/scripts/**`

Compatibility note:
- Canonical docs term is `Shell`.
- Repository path and guard IDs remain `client/**` until explicit tooling migration is approved.

## Rules

1. No whitespace in any path segment.
2. Directory segments must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
3. File stems must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
4. Uppercase filenames are blocked except explicit allowlist entries (`README.md`, `QUICKREF.md`, `GOVERNANCE.md`).
5. Generic code stems are blocked for code files:
   - `util`, `utils`, `helper`, `helpers`, `misc`, `temp`, `tmp`, `newfile`
6. Svelte route special stems are allowed:
   - `+layout`, `+page`, `+server`, `+error`, `+layout.server`, `+page.server`

## Automation

- Guard script: `tests/tooling/scripts/ci/client_naming_policy_guard.ts`
- Policy config: `client/runtime/config/client_naming_policy.json`
- npm command (advisory/yellow flag): `npm run -s ops:client-naming:guard`
- npm command (strict/blocking): `npm run -s ops:client-naming:guard:strict`
- tooling registry gate id: `ops:client-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Artifacts

- JSON: `core/local/artifacts/client_naming_policy_guard_current.json`
- Markdown: `local/workspace/reports/CLIENT_NAMING_POLICY_GUARD_CURRENT.md`
