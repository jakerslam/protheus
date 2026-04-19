# Kernel Naming Policy (Core Path Compatibility)

## Goal

Keep kernel/backend surfaces readable and auditable by enforcing stable naming conventions in CI.

Canonical public term is `Kernel`.  
`Core` remains a compatibility alias while repository paths and guard IDs transition.

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
- Policy config: `client/runtime/config/core_naming_policy.json`
- npm command: `npm run -s ops:core-naming:guard`
- tooling registry gate id: `ops:core-naming:guard`
- CI workflow integration: `.github/workflows/ci.yml` (`Policy Baseline Contract` step)

## Transition Indicators

- [x] Public docs use `Kernel` as the canonical authority term.
- [x] This policy declares `Core` as compatibility alias only.
- [x] Primary guard command renamed to `ops:kernel-naming:guard` (keep `ops:core-naming:guard` compatibility alias).
- [ ] Policy/artifact filenames migrated from `core_naming_*` to `kernel_naming_*` with compatibility mirrors.
- [x] Alias retirement target published in release terminology policy (`v0.5.0` / `2026-07-15`).

## Artifacts

- JSON: `core/local/artifacts/core_naming_policy_guard_current.json`
- Markdown: `local/workspace/reports/CORE_NAMING_POLICY_GUARD_CURRENT.md`
