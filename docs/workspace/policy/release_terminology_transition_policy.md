# Release Terminology Transition Policy

## Purpose

Define release-governed transition rules for canonical public terminology while preserving backward compatibility during migration windows.

## Canonical Terms

- Authority layer: `Kernel` (compatibility alias: `Core`)
- External boundary layer: `Gateways` (compatibility alias: `Adapters`)

## Compatibility Window

- Repository path compatibility remains:
  - `core/**` for kernel authority
  - `adapters/**` for gateway boundary layer
- Legacy aliases are supported until retirement targets below.

## Alias Retirement Targets

- `Core` compatibility alias retirement target:
  - version: `v0.5.0`
  - date: `2026-07-15`
- `Adapters` compatibility alias retirement target:
  - version: `v0.5.0`
  - date: `2026-07-15`

## Release Checklist Requirements

For each release candidate:

1. Public docs must prefer canonical terms (`Kernel`, `Gateways`).
2. Compatibility aliases must be explicitly labeled as aliases when present.
3. Command aliases must remain functional during compatibility window:
   - `ops:kernel-naming:guard` -> `ops:core-naming:guard`
   - `ops:gateway-runtime-chaos:gate` -> `ops:adapter-runtime-chaos:gate`
   - `test:ops:gateway-chaos:rust` -> `test:ops:adapter-chaos:rust`
   - `ops:orchestration:gateway-fallback:guard` -> `ops:orchestration:adapter-fallback:guard`
4. Compatibility mapping file must be present and current:
   - `client/runtime/config/kernel_transition_alias_map.json`
   - `client/runtime/config/gateway_transition_alias_map.json`
5. After retirement target date/version, releases must remove deprecated aliases unless an explicit blocker exception is documented.

## Exception Policy

If an alias cannot be retired by target:

1. Record blocker + owner + new date in release notes.
2. Add explicit exception note in README transition section.
3. Keep alias in compatibility map with `status=extended`.
