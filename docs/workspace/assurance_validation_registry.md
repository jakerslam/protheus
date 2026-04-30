# Assurance Validation Registry

Owner: Kernel / Assurance / Validation
Status: initial seed
Config: `validation/conformance/contracts/assurance_validation_registry.json`
Schema: `validation/schemas/assurance_validation_registry.schema.json`
Covers: `ASSURANCE-009` through `ASSURANCE-012`

## Purpose

The Validation registry is the Assurance-owned index of controlled checks. It separates behavioral proof from execution harnesses and gives each check a lifecycle state.

The older `tests/tooling/**` registry and schema paths are compatibility mirrors only. Active Assurance guards should read the Validation-owned contracts.

This registry does not replace the existing tooling gate registry or test maturity registry. It sits above them:

- `tooling_gate_registry.json` says how to run registered tooling.
- `test_maturity_registry.json` classifies temporary scaffolding and runtime crutches.
- `assurance_validation_registry.json` says what Validation owns as controlled proof, how mature each proof family is, and whether it is behavioral truth or harness-only plumbing.

## Lifecycle States

| State | Meaning |
|---|---|
| `experimental` | Check is being shaped and may not block by itself. |
| `advisory` | Check can produce review or issue-candidate signals but cannot block by itself. |
| `release_gate` | Check can block release, promotion, or unsafe operation through Governance. |
| `retirement_candidate` | Check has met or is near declared retirement criteria and should emit cleanup work instead of becoming permanent scaffolding. |
| `retired` | Check is no longer active; retained only for historical reference. |

## Harness Separation Rule

A harness can run a proof, but it is not the proof.

`harness_only` entries may prove execution plumbing, command discovery, or artifact routing. They must not become release truth, scorecard truth, or behavior truth unless a non-harness Validation entry references their output.

## Initial Registered Families

| ID | Kind | Lifecycle | Signal | Harness-only | Owner of truth |
|---|---|---|---|---:|---|
| `validation.rust_core_regressions` | `regression_suite` | `release_gate` | `hard_gate` | false | `core/**` Rust tests |
| `validation.eval_quality` | `eval_suite` | `release_gate` | `hard_gate` | false | Assurance Validation eval definitions |
| `validation.public_benchmark_harness` | `benchmark` | `advisory` | `advisory` | false | `benchmarks/public_harness` |
| `validation.architecture_conformance` | `conformance_guard` | `release_gate` | `hard_gate` | false | architecture policy guards |
| `validation.shell_projection_conformance` | `conformance_guard` | `release_gate` | `hard_gate` | false | Shell projection policies and guards |
| `validation.gateway_boundary_conformance` | `conformance_guard` | `release_gate` | `hard_gate` | false | Gateway and Conduit boundary policies |
| `validation.release_proof_pack` | `release_proof_check` | `release_gate` | `hard_gate` | false | release proof-pack assembly |
| `validation.temporary_scaffolding_maturity` | `conformance_guard` | `retirement_candidate` | `advisory` | false | test maturity registry |
| `harness.tooling_registry_runner` | `harness_only` | `advisory` | `diagnostic` | true | tooling runner infrastructure |

## Temporary Scaffolding Rule

Temporary scaffolding and temporary monitors must include:

- `retirement_criteria`;
- `retirement_action`;
- `strengthen_signal`.

Default retirement criteria are:

```json
{
  "minimum_runs": 30,
  "success_rate_required": 0.98,
  "observation_window_days": 14,
  "consecutive_passes_required": 10,
  "max_regressions_allowed": 0
}
```

When these criteria are met, the check should not delete itself. It should emit a retirement backlog item so a human or Governance process can remove, merge, or downgrade the scaffold intentionally.

## Strengthening Rule

If a temporary check persists past its retirement window, the correct response is not more scaffolding by default.

The registry must name the weak runtime mechanism using `strengthen_signal`, then route improvement work to the owning layer.
