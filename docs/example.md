# Layer 2 Initiative Extension Example

This is a minimal example for introducing a custom lane behavior while preserving deterministic scoring.

## Example Goal

Add a `regulatory_risk` signal that boosts urgency for regulated workflows.

## Patch Pattern

1. Extend input:
```rust
pub struct ImportanceInput {
    // existing fields...
    pub regulatory_risk: Option<f64>,
}
```

2. Include in weighted score with bounded contribution.
3. Keep `score` clamped to `0..=1`.
4. Preserve existing score-band thresholds unless explicitly versioning.
5. Add tests for:
   - bounds (`score` never outside `0..=1`),
   - monotonicity (higher risk should not reduce score),
   - stable ordering in `prioritize_attention_json`.

## Verification Commands

```bash
cargo test --manifest-path core/layer2/execution/Cargo.toml initiative
cargo test --manifest-path core/layer0/ops/Cargo.toml importance
```
