# Formal Invariant Engine (`V3-045`)

`client/runtime/systems/security/formal_invariant_engine.ts` is the machine-checkable invariant lane.

## Spec

Invariant spec file:

- `client/runtime/config/formal_invariants.json`

Supported deterministic invariant types:

- `file_contains_all`
- `json_path_exists`
- `json_path_equals`
- `json_path_gte`
- `json_path_includes`
- `json_path_one_of`

## Commands

```bash
npm run -s formal:invariants:run
node client/runtime/systems/security/formal_invariants_bootstrap.ts run --strict=1
node client/runtime/systems/security/formal_invariant_engine.ts status
```

`formal:invariants:run` now auto-installs `typescript` (`npm install --no-save typescript`) when missing, so clean validation worktrees do not fail on bootstrap.

## Outputs

- Latest result: `client/runtime/local/state/security/formal_invariant_engine/latest.json`
- History: `client/runtime/local/state/security/formal_invariant_engine/history.jsonl`

## CI Wiring

Merge guard enforces this lane:

- `client/runtime/systems/security/merge_guard.ts` runs `formal_invariant_engine.js run --strict=1`

This turns core sovereignty/non-bypass checks into explicit executable invariants rather than prose-only policy.
