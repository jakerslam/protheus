# Shell Naming Policy

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

Path note:
- Canonical docs term is `Shell`.
- Repository path remains `client/**` until explicit tooling migration is approved.

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

- Guard script: `tests/tooling/scripts/ci/naming_policy_guard.ts`
- Policy config (canonical): `client/runtime/config/shell_naming_policy.json`
- npm command (canonical advisory/yellow flag): `npm run -s ops:shell-naming:guard`
- npm command (canonical strict/blocking): `npm run -s ops:shell-naming:guard:strict`
- tooling registry gate id: `ops:shell-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Artifacts

- JSON (canonical): `core/local/artifacts/shell_naming_policy_guard_current.json`
- Markdown (canonical): `local/workspace/reports/SHELL_NAMING_POLICY_GUARD_CURRENT.md`
