# Orchestration Naming Policy

## Goal

Keep orchestration surfaces readable and auditable through stable naming conventions and continuous tracking.

Current enforcement mode: **Yellow Flag (advisory)**.
- Naming violations are surfaced in CI artifacts and reports.
- They are not merge-blocking by default.
- Strict blocking remains available as an explicit opt-in lane.

## Enforced Scope

The guard applies to:

- `surface/orchestration/src/**`
- `surface/orchestration/tests/**`
- `surface/orchestration/scripts/**`

## Rules

1. No whitespace in any path segment.
2. Directory segments must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
3. File stems must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
4. Uppercase filenames are blocked except explicit allowlist entries (`README.md`, `QUICKREF.md`, `GOVERNANCE.md`).
5. Generic code stems are blocked for code files:
   - `util`, `utils`, `helper`, `helpers`, `misc`, `temp`, `tmp`, `newfile`

## Automation

- Guard script: `tests/tooling/scripts/ci/naming_policy_guard.ts`
- Policy config: `client/runtime/config/orchestration_naming_policy.json`
- npm command (advisory/yellow flag): `npm run -s ops:orchestration-naming:guard`
- npm command (strict/blocking): `npm run -s ops:orchestration-naming:guard:strict`
- tooling registry gate id: `ops:orchestration-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Artifacts

- JSON: `core/local/artifacts/orchestration_naming_policy_guard_current.json`
- Markdown: `local/workspace/reports/ORCHESTRATION_NAMING_POLICY_GUARD_CURRENT.md`
