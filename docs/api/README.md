# API Reference

This directory is the canonical API reference surface for InfRing/Infring runtime operators and integrators.

## Scope

- CLI command surfaces (`infring-ops`, `infringd`)
- Runtime bridge contracts (client runtime lane wrappers)
- State/receipt artifact interfaces
- Compatibility and versioning notes

## Primary Interfaces

1. `infring-ops` command families
2. `infringd` daemon command/router surfaces
3. Thin runtime wrappers under `client/runtime/systems/**`
4. Adapter bridges under `adapters/**`

## Source-of-Truth Pointers

- Ops CLI usage: `core/layer0/ops/src/ops_main_usage.rs`
- Route dispatch: `core/layer0/ops/src/infringctl_routes.rs`
- Runtime wrappers: `client/runtime/systems/`
- Rust authority contracts: `core/layer0/ops/src/contract_check.rs`

## Response Shape Conventions

Most command/status surfaces return JSON with:

- `ok` (`true`/`false`)
- `type` (lane/type discriminator)
- `claim_evidence` (when contract-backed)
- `receipt_hash` (deterministic receipt hash)
- `strict` (if strict mode evaluated)

## Error Model

Fail-closed lanes typically emit:

- `ok: false`
- `type: <lane>_error` or lane-specific denial type
- `errors: [...]` and/or `code`

## Versioning and Compatibility

- Compatibility-sensitive lanes should pin explicit versions in payloads/contracts.
- Backward-compatibility controls must fail closed in strict mode.

## Authoritative OpenAPI

- Dashboard runtime API spec: [openapi.stub.yaml](./openapi.stub.yaml)
- Runtime authority: `core/layer0/ops/src/dashboard_ui.rs` (`/healthz`, `/api/dashboard/snapshot`, `/api/dashboard/action`)

## Operator Cross-Links

- Operator index: [docs/ops/INDEX.md](../ops/INDEX.md)
- Security policy: [SECURITY.md](../../SECURITY.md)
- Deployment procedures: [RUNBOOK-002](../ops/RUNBOOK-002-deployment-procedures.md)
- ADR registry: [docs/client/adr/INDEX.md](../client/adr/INDEX.md)
