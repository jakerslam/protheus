# REQ-33 — Cockpit Push Stream + Layer Wrapper Contracts

Status: implemented (2026-03-08)
Owner: runtime-core

## Scope

1. Add non-blocking origin-integrity startup behavior for `infringd` so timeout stalls do not hard-fail ambient runtime startup.
2. Add attach-first cockpit entry and reliable subscribe behavior for long-lived ambient operation.
3. Add long-poll attention delivery contract (`--wait-ms`) for conduit-safe event stream semantics.
4. Implement executable Layer `-1` and Layer `3` wrapper crates for architecture conformance.

## Requirements

- `infringd` must allow degraded startup when origin-integrity fails only due timeout-like reasons and policy allows degraded timeout start.
- `infringd` must persist retry metadata (`pending`, `next_retry_at`) and retry origin-integrity checks in ambient loop.
- `infringd` default command must be `attach` unless explicitly overridden.
- `infringd subscribe` must degrade gracefully on transient conduit timeout and keep stream alive.
- `attention-queue drain/next` must support `--wait-ms` for long-poll retrieval.
- Workspace must include executable wrapper crates:
  - `core/layer_minus_one/exotic_wrapper`
  - `core/layer3/os_extension_wrapper`
- `kernel_layers` must expose Layer `-1` and Layer `3` wrappers behind compile-time features.

## Delivered Artifacts

- `client/runtime/systems/ops/infringd.ts`
- `core/layer0/ops/src/attention_queue.rs`
- `core/layer_minus_one/exotic_wrapper/*`
- `core/layer3/os_extension_wrapper/*`
- `core/layer0/kernel_layers/*`
- `Cargo.toml`
- `ARCHITECTURE.md`
- `docs/workspace/SRS.md`
