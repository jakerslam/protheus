# Surface Compression Foundation (2026-05-10)

This pass addresses three adoption/maintenance risks:

1. `package.json` script sprawl.
2. Oversized installer scripts.
3. Stale root-level AI analysis clutter.

## Package script surface

Current state:

- `package.json` script count: 1140 after adding canonical runner/guard entrypoints.
- Command truth is mirrored into `tools/commands/command_registry.json`.
- Compatibility aliases remain in `package.json` for now.

New canonical entrypoints:

- `npm run -s cmd -- <command-id> [args...]`
- `npm run -s commands:list`
- `npm run -s commands:groups`
- `npm run -s ops:command-surface:guard`

Guard:

- `tests/tooling/scripts/ci/package_script_surface_guard.ts`
- Policy: `validation/conformance/contracts/package_script_surface_policy.json`

The guard prevents silent script-surface growth and requires the registry to cover all package scripts.

## Installer surface

Current state:

- `install.sh`: 194657 bytes.
- `install.ps1`: 175226 bytes.

New module contracts:

- `install/modules/bootstrap_contract.json`
- `install/modules/repair_contract.json`
- `install/modules/windows_wrapper_contract.json`

Guard:

- `tests/tooling/scripts/ci/installer_surface_guard.ts`
- Policy: `validation/conformance/contracts/installer_surface_policy.json`

The guard enforces syntax checks, module contracts, token checks, and no size growth beyond the current compatibility baseline. The shrink target is 50KB per installer.

## Stale analysis cleanup

Deleted root-level `INFRING_REPO_ANALYSIS.md` so it cannot act as false authority.

## Validation run

```bash
npm run -s ops:command-surface:guard
npm run -s ops:installer-surface:guard
npm run -s commands:groups
```
