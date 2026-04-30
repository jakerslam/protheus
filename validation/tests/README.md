# Validation tests

This subdomain is part of the physical Validation domain. It owns controlled test definitions, test-suite metadata, lifecycle contracts, fixtures, schemas, and report destinations.

Canonical lifecycle registry:

- `contracts/validation_test_lifecycle_registry.json`

Canonical suite lifecycle manifest:

- `contracts/test_suite_lifecycle_manifests.json`

Canonical lifecycle artifact envelope contract:

- `contracts/test_lifecycle_artifact_envelope_contract.json`

Canonical temporary test lifecycle contract:

- `contracts/temporary_test_lifecycle_contract.json`

Canonical physical placement policy:

- `contracts/test_lifecycle_physical_placement_policy.json`

Canonical unregistered-test policy:

- `contracts/unregistered_test_policy.json`

Compatibility mirror status:

- retired by `ASSURANCE-DEBT-034`; lifecycle consumers must use `contracts/validation_test_lifecycle_registry.json`.

Lifecycle-managed tests and gates must declare classification, owner/runtime owner, invariant, evidence artifact, strengthening signal, and retirement criteria when temporary. Suite manifests group related checks and must declare `classification`, `owner`, `runtime_owner`, `invariant`, `evidence_artifact`, `lifecycle_state`, and `temporary`. Lifecycle artifacts should expose normalized envelopes with `ok`, `test_id`, `classification`, `invariant`, `runtime_owner`, `failure_signature`, and `strengthen_signal`. Temporary scaffolds, monitors, crutches, and one-time closure guards must also declare `why_it_exists`, `runtime_weakness_signal`, `target_runtime_owner`, `expires_after`, `migration_target`, `delete_when`, and numeric success criteria. Canonical lifecycle definitions belong under `validation/**`; `tests/tooling/**` is harness-only execution infrastructure or explicitly registered harness-only exemptions. New gates must be lifecycle-registered, declared harness-only, or covered by an owner/expiry exemption.

Harness code may remain under `tests/tooling/**`, but lifecycle truth belongs here.

Migration status: canonical lifecycle registry and suite lifecycle manifest established; compatibility mirror burn-down is complete; remaining work is continuous registration enforcement for every new test, gate, benchmark, eval, and conformance suite.
