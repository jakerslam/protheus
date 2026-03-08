# Adaptive Layer

Purpose: keep policy/state that should evolve from evidence, while leaving `client/runtime/systems/` as stable execution + safety infrastructure.

## Sub-layers

- `client/cognition/adaptive/reflex/`
  Fast micro-routine policy and tuning notes.
- `client/cognition/adaptive/client/cognition/habits/`
  Repeat-derived routine lifecycle policy.
- `client/cognition/adaptive/strategy/`
  Strategy scoring/promotion policy and learned scorecards.

## Boundary

- `client/cognition/adaptive/*` stores changeable policy + learned state shape.
- `client/runtime/systems/*` enforces deterministic gates, execution, and security controls.
- Domain-specific implementations remain in `client/cognition/skills/` or `client/cognition/habits/`.
