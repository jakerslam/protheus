# Shell Naming Policy (compat path: `client/**`)

## Goal

Keep shell surfaces readable at scale by enforcing stable, predictable naming and blocking ambiguous additions in CI.

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
- npm command: `npm run -s ops:client-naming:guard`
- tooling registry gate id: `ops:client-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Artifacts

- JSON: `core/local/artifacts/client_naming_policy_guard_current.json`
- Markdown: `local/workspace/reports/CLIENT_NAMING_POLICY_GUARD_CURRENT.md`
