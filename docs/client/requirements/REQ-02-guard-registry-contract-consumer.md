# REQ-02: Guard Registry Contract Consumer

Version: 1.0  
Date: 2026-03-15  
Owner: Infring Kernel / Runtime Security

## Objective

Ensure all runtime guard decisions consumed by client/runtime surfaces are loaded from a deterministic registry contract, never hard-coded ad hoc. This keeps Rust-core authority intact while preserving thin-client behavior.

## Scope

In scope:
- Canonical guard registry loading for runtime/client guard consumers
- Fail-closed behavior when a required guard contract is missing or malformed
- Deterministic receipt emission for guard resolution decisions
- Contract parity checks between configured guards and runtime consumers

Out of scope:
- Defining new guard semantics in this requirement (handled by guard-producing requirements)
- Replacing Layer-0 safety authority with client-side policy logic

## Requirements

### REQ-02-001: Deterministic Guard Registry Resolution

**Requirement:** Guard consumers must resolve contracts from canonical registry artifacts.

**Acceptance:**
- Runtime consumers load guard metadata from canonical registry/config paths
- Missing registry entries cause fail-closed denial behavior
- Registry resolution is deterministic for equivalent inputs

### REQ-02-002: Fail-Closed Contract Consumption

**Requirement:** Guard consumers reject unknown or malformed contracts.

**Acceptance:**
- Malformed contract payloads return deterministic error codes
- Unknown guard IDs are denied by default
- Consumer paths do not silently fall back to permissive defaults

### REQ-02-003: Evidence and Regression Coverage

**Requirement:** Guard consumer contract behavior is test-covered and auditable.

**Acceptance:**
- Regression tests validate deterministic resolution, malformed payload rejection, and fail-closed defaults
- CLI verification path exists for operator-side contract checks
- Evidence links resolve to implementation files and tests (not backlog-only references)

## Verification

- `npm run -s ops:control-plane:audit`
- `npm run -s test:memory:client-guards`
- `npm run -s ops:srs:full:regression`

## Notes

- Rust core remains authority for guard enforcement; client/runtime remains a strict consumer boundary.
