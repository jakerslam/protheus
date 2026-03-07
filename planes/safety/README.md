# Safety Plane

The safety plane owns deterministic correctness, authorization, and recovery.

## Scope

- Scheduling, isolation, capability enforcement.
- Conduit server + scrambler boundary enforcement.
- Constitution, receipts, audit lineage, and fail-closed policy gates.
- Attention queue escalation authority.
- Memory/dopamine/persona/spine ambient policy authority.

## Reinterpreted OS Primitives

- Process -> Agent cell (isolated code + state + authority).
- Thread -> Task (deadline + budget + effect profile).
- Syscall -> Effect-typed capability invocation.
- Interrupt -> Typed event stream.
- Driver -> Substrate adapter.

## Consent Kernel Stub

The consent kernel lives in safety authority and separates permissions for:

- observe
- infer
- feedback
- stimulate

This boundary is mandatory for future neural I/O surfaces.

## Degradation Contract

Safety must always provide deterministic fallback paths when cognition/substrate services degrade.
See `planes/substrate/degradation_contract.schema.json`.
