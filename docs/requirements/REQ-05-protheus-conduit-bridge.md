# REQ-05 Protheus Conduit Bridge

Version: 1.0
Date: 2026-03-05

## Purpose

Define a narrow, typed, fail-closed bridge between Rust core (TCB) and TypeScript surfaces so UI/marketplace/extensions can stay flexible without compromising portability and sovereignty.

## Goals

- Preserve Rust-first invariants for constitution/policy/receipt enforcement.
- Keep TS optional and removable for kernel/bare-metal modes.
- Support low-latency hosted operation (`<5ms` round trip target).
- Ensure deterministic claim-evidence logging for every crossing.

## Scope

In scope:

- `crates/conduit` Rust crate
- `systems/conduit/conduit-client.ts` typed TS client
- Typed JSON schema for Unix socket + stdio
- Rust-side validation and policy gate enforcement
- Deterministic crossing receipts

Out of scope:

- Shared memory/direct function call bypass
- TS-owned persistent core state
- Complex RPC frameworks

## Protocol (10 core messages)

TS -> Rust commands:

1. `start_agent`
2. `stop_agent`
3. `query_receipt_chain`
4. `list_active_agents`
5. `get_system_status`
6. `apply_policy_update` (constitution-safe only)
7. `install_extension`

Rust -> TS events/responses:

8. `agent_started` / `agent_stopped`
9. `receipt_added`
10. `system_status` / `policy_violation`

## Validation Requirements

- Rust is source of truth for schema and policy checks.
- Invalid input or denied policy evaluates fail-closed.
- `apply_policy_update` requires `constitution_safe/*` patch IDs.
- `install_extension` requires valid sha256 + explicit capabilities.

## Transport Requirements

- Primary: Unix domain socket (hosted)
- Fallback: stdio (embedded/lightweight)

## Test Requirements

- Unit tests for schema, validation, and deterministic hashes.
- Stdio round-trip test.
- Invariant tests for fail-closed behavior.

## Initial Delivery (Phase 1)

Delivered in this increment:

- `crates/conduit` scaffold + typed schema
- deterministic receipt hashing
- Rust-side validation gate framework
- Unix socket + stdio transport handlers
- TS typed client scaffold

## Phase 2 Delivery (Constitution + Runtime Policy Binding)

Delivered:

- `RegistryPolicyGate` in `crates/conduit` binds command validation to:
  - `AGENT-CONSTITUTION.md` required markers
  - `config/guard_check_registry.json` required runtime checks
- deny-by-default capability mapping via `ConduitPolicy.command_required_capabilities`
- deterministic `policy_receipt_hash` emitted on every request

## Must-Have Security Hardening Delivery

Delivered:

1. Message Signing
   - `crates/conduit-security` `MessageSigner`
   - every command verifies deterministic signature before execution
2. Capability Tokens
   - `CapabilityTokenAuthority` mint/validate in Rust
   - command-specific capability scope enforcement
3. Rate Limiting
   - `RateLimiter` per-client and per-client-command windows
   - fail-closed `policy_violation` path on throttle

## Phase 3 Delivery (Feature Migration Path)

Delivered:

- `systems/ops/protheusd.ts` supports conduit-first routing for:
  - `start`
  - `stop`
  - `status`
- graceful fallback to legacy control-plane route when conduit is unavailable

## Phase 4 Delivery (Certification Gate)

Delivered:

- `crates/conduit/tests/certification.rs`
  - parity check: direct core path vs stdio path
  - hosted average round-trip budget assertion (`<5ms`)
  - embedded stdio budget assertion (`<20ms`)

## Phase 5 Delivery (Rust Source-of-Truth Enforcement)

Delivered:

- `systems/ops/protheusd.ts` now defaults lifecycle commands (`start`, `stop`, `status`) to strict conduit routing.
  - strict failure path: `conduit_required_strict:*`
  - explicit temporary escape hatch only via `--allow-legacy-fallback` or `PROTHEUS_ALLOW_LEGACY_FALLBACK=1`
- `config/rust_source_of_truth_policy.json` defines source-of-truth contract checks for:
  - Rust kernel entrypoints in `crates/ops/src/main.rs`
  - strict conduit enforcement tokens in `systems/ops/protheusd.ts`
  - conduit message-budget enforcement tokens in `crates/conduit/src/lib.rs`
  - required JS bootstrap wrappers
  - required direct JS->Rust shims (token-validated)
- `crates/ops/src/contract_check.rs` enforces `rust_source_of_truth_contract` as a first-class contract check.
- bridge contract budget is enforced fail-closed at runtime:
  - max bridge message types: `10`
  - TS commands: `7`
  - Rust event types: `3`
