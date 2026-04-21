# Layer 3 Contract (Kernel OS Personality Surface)

Status: canonical contract for `core/layer3/**` additions.

## Purpose

Layer 3 hosts OS-personality surfaces on top of Layer 2 execution guarantees without taking scheduling truth away from Layer 2.

## Authority and Boundary Rules

- Layer 2 remains authoritative for scheduling, admission, and execution-lane truth.
- Layer 3 may define process/service abstractions, but must consume Layer 2 receipts and boundaries.
- Layer 3 must not introduce parallel scheduler truth, queue authority, or policy authority that bypasses Layer 2.
- Every Layer 3 module must be declared in `tests/tooling/config/layer3_contract_policy.json`.

## Required Module Fields

Every Layer 3 module policy row must include:

- `id`
- `path_prefix`
- `category`
- `status` (`complete` or `experimental`)
- `owner`
- `timeout_semantics`
- `retry_semantics`
- `receipt_requirements`
- `parity_test_path`
- `scheduler_boundary.layer2_interface`
- `scheduler_boundary.authority`
- `execution_unit.id`
- `execution_unit.lifecycle`
- `execution_unit.budget`
- `execution_unit.dependencies`
- `execution_unit.receipts`

## Execution-Unit Model (Minimum)

Every Layer 3 execution unit must define and preserve:

- `id`: stable identifier
- `lifecycle`: allowed states and transitions
- `budget`: bounded resources inherited from Layer 2 guarantees
- `dependencies`: explicit lower-layer dependencies
- `receipts`: emitted runtime proofs for lifecycle/boundary transitions

## Category Taxonomy

Allowed categories for Layer 3 modules:

- `process`
- `service`
- `vfs`
- `driver`
- `syscall`
- `namespace`
- `networking`
- `windowing`

## CI Enforcement

Guard: `ops:layer3:contract:guard`

Authoritative files:

- policy: `tests/tooling/config/layer3_contract_policy.json`
- guard: `tests/tooling/scripts/ci/layer3_contract_guard.ts`

Failure mode is fail-closed:

- Unmapped Layer 3 source files fail.
- Missing execution-unit or scheduler-boundary fields fail.
- Invalid category/status fail.
