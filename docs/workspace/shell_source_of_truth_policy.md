# Shell Source-Of-Truth Policy

Owner: Shell Runtime
Effective: 2026-04-28

## Purpose

Define which Shell files are canonical authored source, which are decomposition artifacts, and which are generated delivery artifacts so metrics, guards, and cleanup waves do not treat one logical surface as multiple independent production codebases.

This policy applies first to `client/runtime/systems/ui/infring_static/**` and any Shell-adjacent tooling or guard that audits tracked TypeScript surface area.

## Canonical Categories

### 1. Canonical Authored Source

These files are the human-edited source of truth for Shell behavior and should count as production source:

- normal authored `*.ts` / `*.tsx` Shell modules
- `*_svelte_source.ts` files for Svelte shell islands
- canonical assembled files that are still the runtime entry surface during migration, such as `app.ts` and `pages/chat.ts`, until they are replaced by real module graphs

### 2. Decomposition Artifacts

These files exist to split a large logical surface into manageable shards during migration, but they are not an additional independent codebase:

- `*.parts/**`
- segmented `*.partXX.ts` and similar decomposition shards under a logical parent

Rules:

- A decomposition artifact must have one logical parent surface.
- Metrics and size reports must not count the logical parent and all decomposition shards as additive production source.
- Guards may inspect decomposition artifacts for ownership and migration debt, but they must treat them as part of the parent surface.
- New decomposition artifacts are allowed only as time-bounded migration debt, not as a permanent second representation.

### 3. Generated Delivery Artifacts

These files are output forms used by the live Shell or local packaging flow, but they are not canonical authored source:

- `*.bundle.ts`
- other generated shell delivery outputs produced from canonical source

Rules:

- Generated delivery artifacts must map back to one canonical source file.
- Metrics that measure authored Shell source growth must exclude generated delivery artifacts.
- Tracking generated delivery artifacts in git is allowed only when the runtime or packaging flow still requires them.
- Generated delivery artifacts must never become the only discoverable source of Shell behavior.

## Canonical Pairing Rules

### Svelte Shells

For Svelte shell islands:

- canonical source: `*_svelte_source.ts`
- generated delivery artifact: `*.bundle.ts`

The bundle is a delivery form, not a second authored module.

### Assembled Files With `.parts/**`

For assembled Shell files that also have `.parts/**` mirrors:

- exactly one logical surface is canonical
- the `.parts/**` tree is decomposition debt
- metrics and cleanup reports must count the logical surface once

During migration, the canonical logical surface may still be represented by the assembled file, but the decomposition shards must not inflate authored-source growth numbers.

## Metric Rules

The following reports and guards must follow this policy:

- effective LoC and Shell size reporting
- duplicate-surface inventory
- Shell cleanup reports
- future Shell source-of-truth guards

Required behavior:

1. Canonical authored source counts as production source.
2. Decomposition artifacts are counted as migration structure, not additive production source.
3. Generated delivery artifacts are excluded from authored-source growth metrics.
4. Duplicate reports must estimate avoidable duplicate LoC by logical surface, not just by raw file count.

## Migration Rules

When cleaning up Shell surfaces:

1. Prefer moving from assembled-plus-parts to real modules.
2. Prefer one canonical authored source per behavior surface.
3. Delete generated or duplicate tracked artifacts only after the runtime and packaging path prove they are no longer needed.
4. If a large Shell surface still needs decomposition shards temporarily, record that as migration debt and keep the parent-child mapping explicit.

## Immediate Implications

Based on the current duplicate inventory:

- `pages/chat.ts` and `pages/chat.ts.parts/**` are one logical surface, not two
- `app.ts` and `app.ts.parts/**` are one logical surface, not two
- `*_svelte_source.ts` is the authored source for Shell Svelte islands
- `*.bundle.ts` is a generated delivery artifact and should not drive authored-source size claims

## Required Follow-On Work

- `SHELL-CLEANUP-003` must update effective LoC and related Shell metrics to honor this policy.
- `SHELL-CLEANUP-004` must classify or exclude generated Shell delivery artifacts in authored-source metrics.
- `SHELL-CLEANUP-005` and `SHELL-CLEANUP-006` must collapse the `chat.ts` and `app.ts` dual-representation debt.
