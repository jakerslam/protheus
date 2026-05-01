# Kernel Sentinel Feedback Quality

Owner: Kernel Sentinel model with Observability review records.

Kernel Sentinel feedback quality tracks whether a Sentinel finding became useful learning material or merely another symptom patch.

## Review statuses

- `accepted`: finding was valid and should reinforce Sentinel judgment.
- `rejected`: finding was not useful and should remain audit-only.
- `actionable`: finding is valid work with evidence, root cause, and next action.
- `resolved`: finding was fixed and has a resolution reference.
- `symptom_patch`: finding or remediation treated a symptom while missing root cause.

## Learning requirements

Sentinel feedback can strengthen the system only when it carries:

- evidence refs
- root-cause hypothesis
- concrete next action

Weak accepted/actionable feedback is treated as symptom-patch risk until reviewed.

## Kernel model

The authoritative model lives in:

`core/layer0/ops/src/kernel_sentinel/feedback_quality.rs`

Validation:

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib feedback_quality -- --nocapture
```
