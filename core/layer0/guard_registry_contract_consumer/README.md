# Guard Registry Contract Consumer

`guard_registry_contract_consumer` is a small Kernel utility crate for reading
and normalizing guard-registry contract rows into deterministic authorization
queries.

## Responsibility

- Normalize guard ids and capability tokens into deterministic ASCII-safe forms.
- Merge duplicate guard rows without losing declared capabilities.
- Fail closed for empty, invalid, inactive, or capability-less guards.
- Answer whether a normalized guard id authorizes a normalized capability.

## Non-Goals

- It does not own the canonical guard registry.
- It does not perform runtime admission by itself.
- It does not write guard registry artifacts.

## Validation

Run the crate invariants with:

```bash
cargo test --manifest-path core/layer0/guard_registry_contract_consumer/Cargo.toml
```
