# Release Terminology Canonical Policy

## Purpose

Define release-governed canonical terminology rules after alias retirement.

## Canonical Terms

- Authority layer: `Kernel`
- External boundary layer: `Gateways`
- Presentation layer: `Shell`

## Alias Retirement State

- Kernel/Core compatibility aliases and Gateways/Adapters compatibility aliases are retired effective:
  - version: `retired-2026-04-22`
  - date: `2026-04-22`
- Shell/Client compatibility alias bridges are retired effective:
  - version: `retired-2026-04-24`
  - date: `2026-04-24`
  - guard source: `client/runtime/config/shell_transition_compatibility_bridges.json`
- Legacy repository paths (`core/**`, `adapters/**`, `client/**`) remain implementation paths only.

## Release Checklist Requirements

For each release candidate:

1. Public docs must use canonical-first terminology (`Kernel`, `Gateways`, `Shell`).
2. Retired alias labels must not appear in public/operator docs as layer names.
3. Terminology inventory must fail-closed on retired alias terms in scanned docs.
4. Canonical transition/tracker files must be present and current:
   - `client/runtime/config/kernel_transition_alias_map.json`
   - `client/runtime/config/gateway_transition_alias_map.json`
   - `client/runtime/config/shell_transition_alias_map.json`
   - `client/runtime/config/terminology_transition_deprecation_tracker.json`
5. Public/operator docs must not present any retired authority alias as a standalone primary authority label; canonical form must be `Kernel`.

## Exception Policy (Break-Glass Only)

If a retired alias must be temporarily reintroduced:

1. Record blocker + owner + new date in release notes.
2. Add explicit exception note in README transition section.
3. Reintroduce alias only as a time-bounded break-glass override with explicit expiry.
