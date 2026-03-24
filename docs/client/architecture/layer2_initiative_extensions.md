# Layer 2 Initiative Primitive Extension Guide

This guide defines how to extend initiative behavior without violating safety-plane authority.

## Purpose

`core/layer2/execution/src/initiative.rs` is the public Layer 2 primitive for:
- importance scoring,
- escalation action selection,
- attention queue ordering.

## Contract Rules

- Keep all scores deterministic and bounded (`0..=1`).
- Preserve stable ordering for equal-ranked events.
- Do not mutate Layer 0 safety decisions from Layer 2.
- Treat any new initiative action as policy-visible and receipt-emitting.

## Extension Pattern

1. Add any new metric in `ImportanceInput` as optional and clamped.
2. Extend scoring weights in a backward-compatible way.
3. Add action mapping in `initiative_for_score` with explicit thresholds.
4. Add unit + property tests for score bounds and ordering invariants.
5. Expose JSON lane output with clear `type` and threshold metadata.

## Minimal Example

See [docs/example.md](../../example.md) for a concrete custom lane extension walkthrough.
