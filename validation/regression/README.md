# Validation regression

This subdomain owns controlled regression-suite metadata and release-class workload proof policy.

Canonical regression policy now lives here:

- `policies/runtime_empirical_coverage_policy.json` defines supported profiles and empirical coverage floors for runtime proof reality checks.
- `policies/runtime_soak_scenarios_policy.json` defines required soak scenarios and sample floors for multi-agent, long-running, mixed-workload, and gateway-failure regression checks.
- `compatibility_mirrors.json` declares old tooling config paths that remain temporary mirrors while callers migrate.

Harnesses may live under `tests/tooling/**`, but regression policy truth should be read from this subdomain.
