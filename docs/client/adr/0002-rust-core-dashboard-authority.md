# ADR 0002: Rust-Core Dashboard API Authority

- Status: accepted
- Date: 2026-03-20
- Owners: Jay
- Supersedes: n/a

## Context

The dashboard runtime had mixed ownership signals between client-side UI concerns and runtime action/snapshot authority. This created regression risk around chat action routing, API contract clarity, and operational health checks.

## Decision

The Rust `dashboard-ui` lane in `core/layer0/ops/src/dashboard_ui.rs` is the authoritative API/runtime surface for dashboard operations:

- `GET /healthz`
- `GET /api/dashboard/snapshot`
- `POST /api/dashboard/action`

The client dashboard remains a thin UI consumer of this authority and must not bypass these lane-backed contracts.

## Consequences

- API behavior is centralized, receipted, and fail-closed in Rust core authority.
- Documentation can publish a stable OpenAPI contract for operators and integrators.
- Regression tests target server-side action/snapshot semantics directly.
- Future dashboard UI changes must preserve compatibility with this API contract.
