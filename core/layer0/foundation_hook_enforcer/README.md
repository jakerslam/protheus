# Foundation Hook Enforcer

`foundation_hook_enforcer` is a small Kernel utility crate for proving that
required foundation hooks are present before a guard or merge contract can be
treated as covered.

## Responsibility

- Normalize hook/check identifiers into deterministic ASCII-safe tokens.
- Compare required hook lists against source text or mandatory hook lists.
- Fail closed when the check id, source text, or required hook inventory is
  missing or oversized.
- Emit deterministic `HookCoverageReceipt` values that can be compared in
  tests and downstream guards.

## Non-Goals

- It does not discover hooks from the filesystem.
- It does not own CI policy or release gating.
- It does not mutate guard registries or source files.

## Validation

Run the crate invariants with:

```bash
cargo test --manifest-path core/layer0/foundation_hook_enforcer/Cargo.toml
```
