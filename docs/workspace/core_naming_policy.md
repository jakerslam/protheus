# Kernel Naming Policy (Core Compatibility Alias + Path Compatibility)

## Goal

Keep kernel/backend surfaces readable and auditable through stable naming conventions and continuous tracking.

Current enforcement mode: **Yellow Flag (advisory)**.
- Naming violations are surfaced in CI artifacts and reports.
- They are not merge-blocking by default.
- Strict blocking remains available as an explicit opt-in lane.

Canonical public term is `Kernel`.  
`Core` remains a compatibility alias while repository paths and guard IDs transition.

## Canonicalization Contract

1. Use `Kernel` for all architecture, policy, release, and operator-facing language.
2. Use `Core` only as:
   - explicit compatibility alias (`Kernel` first, `Core` second)
   - immutable repository path prefix (`core/**`) until path migration is explicitly approved.
3. Any new policy/gate/docs content that uses `Core` as primary authority term is non-compliant.

## Enforced Scope

The guard applies to:

- `core/layer0/**`
- `core/layer1/**`
- `core/layer2/**`
- `core/layer3/**`
- `core/layer_minus_one/**`

## Rules

1. No whitespace in any path segment.
2. Directory segments must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
3. File stems must match:
   - `^[a-z0-9]+(?:[._-][a-z0-9]+)*$`
4. Uppercase filenames are blocked except explicit allowlist entries (`Cargo.toml`, `Cargo.lock`, `README.md`, `QUICKREF.md`, `GOVERNANCE.md`, `CHANGELOG.md`, `LICENSE`).
5. Generic code stems are blocked for code files:
   - `util`, `utils`, `helper`, `helpers`, `misc`, `temp`, `tmp`, `newfile`

## Automation

- Guard script: `tests/tooling/scripts/ci/client_naming_policy_guard.ts`
- Policy config (canonical): `client/runtime/config/kernel_naming_policy.json`
- Policy config (compat): `client/runtime/config/core_naming_policy.json`
- npm command (canonical, advisory/yellow flag): `npm run -s ops:kernel-naming:guard`
- npm command (canonical, strict/blocking): `npm run -s ops:kernel-naming:guard:strict`
- npm command (compat, advisory/yellow flag): `npm run -s ops:core-naming:guard`
- npm command (compat, strict/blocking): `npm run -s ops:core-naming:guard:strict`
- tooling registry gate id (compat): `ops:core-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Transition Indicators

- [x] Public docs use `Kernel` as the canonical authority term.
- [x] This policy declares `Core` as compatibility alias only.
- [x] Primary guard command renamed to `ops:kernel-naming:guard` (keep `ops:core-naming:guard` compatibility alias).
- [x] Policy/artifact filenames migrated from `core_naming_*` to `kernel_naming_*` with compatibility mirrors.
- [x] Alias retirement target published in release terminology policy (`v0.5.0` / `2026-07-15`).

## Artifacts

- JSON (canonical): `core/local/artifacts/kernel_naming_policy_guard_current.json`
- Markdown (canonical): `local/workspace/reports/KERNEL_NAMING_POLICY_GUARD_CURRENT.md`
- Compatibility mapping: `client/runtime/config/kernel_transition_alias_map.json`
- Legacy compatibility artifacts: `core/local/artifacts/core_naming_policy_guard_current.json`, `local/workspace/reports/CORE_NAMING_POLICY_GUARD_CURRENT.md`
