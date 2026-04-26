# Shell Alpine Retirement Policy

SRS owner: `V13-CODEX-SHELL-ALPINE-001`

## Rule

No new Alpine bindings, Alpine runtime API calls, or Alpine magic helper usage may be added to the Shell UI.

Existing Alpine usage is treated as migration debt and is frozen against the explicit legacy baseline in `client/runtime/config/shell_alpine_growth_policy.json`.

Every remaining Alpine usage family must also be owned in `client/runtime/config/shell_alpine_ownership_map.json` before removal or migration work continues. Ownership entries must name the Shell feature, the target Svelte/shared shell services replacement, and one migration class: `bootstrap_only`, `interactive`, or `delete_ready`.

## Allowed Direction

New interactive Shell work must be implemented through Svelte/shared shell services. Existing Alpine slices should only move downward in count as they migrate to Svelte components or shared Shell primitives.

## Waivers

A waiver is allowed only when preserving a legacy Alpine path is safer than migrating it immediately. A waiver must name the affected file, explain why Svelte/shared shell services cannot own the behavior yet, and update the baseline in the same change. Waivers are temporary migration debt, not a new default.

## Enforcement

`ops:shell:alpine-growth:guard` compares current Shell UI Alpine usage against the frozen baseline and fails when any file or total pattern count grows.

`ops:shell:alpine-ownership:guard` publishes the living inventory for `V13-CODEX-SHELL-ALPINE-002` and fails when any remaining Alpine binding/store/helper/API family lacks an owner or Svelte/shared-service migration boundary.
