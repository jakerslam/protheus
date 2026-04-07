# REQ-03: Foundation Hook Enforcer

Version: 1.0  
Date: 2026-03-15  
Owner: Protheus Core / Runtime Operations

## Objective

Enforce a deterministic pre-execution foundation hook so every critical runtime path runs baseline safety/integrity checks before execution.

## Scope

In scope:
- Preflight hook enforcement for critical runtime lanes
- Deterministic fail-closed behavior when foundational checks fail
- Explicit operator-visible evidence for hook pass/fail outcomes

Out of scope:
- Replacing feature-specific policy checks
- Bypassing Layer-0 safety controls for convenience paths

## Requirements

### REQ-03-001: Mandatory Foundation Hook Invocation

**Requirement:** Critical lanes must invoke the foundation hook before action execution.

**Acceptance:**
- Hook invocation is required for critical runtime/control-plane routes
- Skipping hook invocation fails execution in strict mode
- Hook invocation path is deterministic and centrally auditable

### REQ-03-002: Fail-Closed Hook Enforcement

**Requirement:** Hook failures must block downstream actions.

**Acceptance:**
- Hook denial returns stable machine-readable reason codes
- Downstream execution is blocked when hook status is non-passing
- No permissive silent bypass is permitted in production mode

### REQ-03-003: Evidence + Regression Validation

**Requirement:** Hook behavior must be backed by tests and operator CLI evidence.

**Acceptance:**
- Regression tests cover pass/fail and bypass-denial scenarios
- CLI evidence path emits deterministic receipts for hook status and denials
- Contract references map to code + tests (not documentation-only claims)

## Verification

- `npm run -s ops:dod:gate`
- `npm run -s ops:srs:full:regression`
- `npm run -s test:memory:client-guards`

## Notes

- This requirement enforces execution discipline; it does not relax Rust-core authority or safety-plane boundaries.
