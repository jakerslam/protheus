# Protheus Architecture

Protheus is a policy-governed local control plane with deterministic receipts.

## Pillars

- Runtime lanes in `systems/`
- Shared runtime helpers in `lib/`
- Contracts/policies in `config/`
- Operator and governance docs in `docs/`

## Core Flow

1. Inputs enter through control surfaces (`protheus`, `protheusctl`, `protheus-top`).
2. Policy gates and safety checks determine allowed execution paths.
3. Runtime lanes execute deterministically and emit receipts under `state/`.
4. Governance lanes verify drift, backlog sync, and documentation integrity.

## Documentation Map

- [Documentation Hub](docs/README.md)
- [Developer Lane Quickstart](docs/DEVELOPER_LANE_QUICKSTART.md)
- [Operator Runbook](docs/OPERATOR_RUNBOOK.md)
- [Security Policy](SECURITY.md)

## Contribution Contract

- Any user-visible change should include tests + docs + changelog evidence.
- Backlog status changes must follow verified implementation.
