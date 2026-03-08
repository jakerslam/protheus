# REQ-29 — Infallible Origin Hardening (Doc Intake)

## Source
- Intake document: `https://docs.google.com/document/d/1sQJowOjNVLpDeixgqXERxB4bEuX9ZvajvJbK2VrBKMQ/edit`
- Captured date: 2026-03-07

## Objective
Make origin integrity an executable gate, not a narrative claim.

## Derived Requirements

### REQ-29-001 Root Invariant Enforcer (executable)
- Add a Rust-authoritative `origin-integrity` lane under `protheus-ops`.
- The lane must emit deterministic receipts and fail closed in strict mode.
- It must validate at least:
  - Conduit-only boundary enforcement via dependency boundary guard.
  - Constitution hardening prerequisites (guardian + resurrection + Merkle config).
  - Safety-plane snapshot hashing over critical root artifacts.

### REQ-29-002 Conduit-only hard gate
- `origin-integrity run` must treat conduit-boundary failure as a hard failure.
- Receipts must include explicit conduit-only claim evidence.

### REQ-29-003 Receipt binding to exact Safety Plane state
- Every origin-integrity receipt must include `safety_plane_state_hash`.
- Receipt must include a deterministic binding hash over checks + safety hash.

### REQ-29-004 Self-audit daemon on start/reload
- `protheusd` must run a startup origin-integrity check with a 30s default timeout.
- If `require_pass_on_start=true`, startup must fail closed when check fails.
- Daemon status/diagnostics must surface the latest origin-integrity result.

### REQ-29-005 Seed bootstrap gate using `verify.sh` certificate
- Add `verify.sh` at repo root as the operator-facing origin verification command.
- Add certificate generation (`origin-integrity certificate`) and peer verification (`origin-integrity seed-bootstrap-verify --certificate=...`).
- Seed bootstrap verification must require matching `verify.sh` hash and safety-plane state hash.

## Out-of-scope / follow-up
- Formal Lean proof extraction integration remains a follow-up hardening lane. Current implementation is deterministic runtime enforcement with receipts.

## Acceptance
- `protheus-ops origin-integrity run --strict=1` succeeds on healthy repo state.
- `protheus-ops origin-integrity certificate --strict=1` emits certificate artifact.
- `protheus-ops origin-integrity seed-bootstrap-verify --certificate=<path>` verifies local/remote match.
- `protheusd start` refuses startup when origin-integrity fails (with strict policy enabled).
