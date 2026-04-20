# Node Critical Path Burndown Plan

## Objective

Reduce Node.js dependence on release-critical operator lanes while preserving release safety and fail-closed behavior.

## Current Source of Truth

- Inventory command:
  - `npm run -s ops:node-critical-path:inventory`
- Artifact:
  - `core/local/artifacts/node_critical_path_inventory_current.json`

## Burndown Stages

1. `inventory`
- Keep deterministic inventory of release-critical scripts.
- Track `node_dependency_ratio` and regression against baseline.

2. `prioritize`
- Prioritize lanes with highest release impact:
  - runtime proof gating
  - release scorecard/verdict
  - proof-pack assembly
  - release closure gate

3. `port_or_wrap`
- Prefer Rust-native authorities for lane logic where feasible.
- If temporary wrappers remain, require explicit compatibility rationale.

4. `re-baseline`
- After each migration wave, refresh baseline artifact and record delta.
- Reject ratio regressions in release lanes without approved exception.

## Success Criteria

1. No regression in `node_dependency_ratio` for release-critical inventory.
2. Each release cycle includes a fresh inventory artifact.
3. Release blockers include unresolved node-critical-path regressions.

