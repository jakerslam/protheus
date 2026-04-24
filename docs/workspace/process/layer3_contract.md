# Layer 3 Contract (Kernel OS Personality Surface)

Status: canonical contract for `core/layer3/**` additions.

## Purpose

Layer 3 hosts OS-personality surfaces on top of Layer 2 execution guarantees without taking scheduling truth away from Layer 2.

Layer 3 is a shape and composition layer. It may describe process, service,
namespace, VFS, driver, syscall, networking, and windowing personalities, but it
must not become a second scheduler, queue, admission controller, policy engine,
or external provider bridge.

## Authority and Boundary Rules

- Layer 2 remains authoritative for scheduling, admission, and execution-lane truth.
- Layer 3 may define process/service abstractions, but must consume Layer 2 receipts and boundaries.
- Layer 3 must not introduce parallel scheduler truth, queue authority, or policy authority that bypasses Layer 2.
- Every Layer 3 module must be declared in `tests/tooling/config/layer3_contract_policy.json`.

## Hard Placement Matrix (Layer 2 vs Layer 3 vs Gateways)

- Layer 2 owns:
  - scheduling truth
  - admission truth
  - execution-lane truth
  - queue authority
  - receipt authority
- Layer 3 owns:
  - process/service/namespace models
  - VFS/driver/syscall/windowing/networking shape abstractions
  - OS personality composition that consumes Layer 2 truth
- Gateways own:
  - external I/O and provider protocol translation
  - non-authoritative bridging at system boundaries
- Layer 3 is forbidden from:
  - direct external provider authority (must go via Gateways)
  - scheduler/admission/queue canonical truth (must remain in Layer 2)

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
- `boundary_alignment.layer2_authority_boundary`
- `boundary_alignment.layer3_scope`
- `boundary_alignment.gateway_boundary`
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

Canonical lifecycle:

- `init`
- `running`
- `degraded`
- `terminated`

Allowed transitions:

- `init -> running`
- `init -> terminated`
- `running -> degraded`
- `running -> terminated`
- `degraded -> running`
- `degraded -> terminated`

Forbidden transitions:

- `terminated -> *`
- any transition without a fresh receipt

Minimum Rust model:

- `ExecutionUnit`
- `ExecutionUnitBudget`
- `ExecutionUnitState`
- `ExecutionUnitTracker`

Current implementation:

- `core/layer3/os_extension_wrapper/src/lib.rs`

The tracker is intentionally minimal: it tracks identity, lifecycle state,
dependencies, budgets, and receipts only. Layer 2 still owns actual scheduling
and admission.

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
- Missing boundary-alignment fields fail.
- Dependency boundary violations (forbidden prefixes or non-allowed prefixes) fail.
- Invalid category/status fail.
