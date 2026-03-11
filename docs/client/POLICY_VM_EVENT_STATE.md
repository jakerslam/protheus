# Policy VM + Event-Sourced State (`V3-044`)

This lane formalizes governance/execution separation and deterministic replay behavior.

## Core Components

- Policy VM: `client/runtime/systems/primitives/policy_vm.ts`
- Canonical event log: `client/runtime/systems/primitives/canonical_event_log.ts`
- Runtime scheduler modes: `client/runtime/systems/primitives/runtime_scheduler.ts`
- N-2 compatibility gate: `client/runtime/systems/ops/profile_compatibility_gate.ts`

## Scheduler Modes

Policy file: `client/runtime/config/runtime_scheduler_policy.json`

Modes are first-class and governed:

- `operational`
- `dream`
- `inversion`

Transitions are policy-bound. Invalid transitions fail closed and are receipted.

## Event-Sourced Canonical State

All primitive execution emits append-only canonical events:

- hash-chained (`prev_hash` / `hash`)
- replay-verifiable (`client/runtime/systems/primitives/replay_verify.ts`)
- scheduler mode transitions emit governance events (`FLOW_GATE`)

## N-2 Compatibility Gate

Policy file: `client/runtime/config/profile_compatibility_policy.json`

`client/runtime/systems/ops/profile_compatibility_gate.ts run --strict=1` enforces schema compatibility:

- capability profile schema version window (N-2)
- primitive catalog schema readability
- fail-closed if profile artifacts drift outside compatibility envelope

This check is wired into merge guard for CI discipline.
