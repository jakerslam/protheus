# Validation benchmarks

This subdomain owns controlled benchmark definitions, performance budgets, and derived benchmark scorecard contracts.

Canonical benchmark and boundedness budget policy now lives here:

- `policies/benchmark_regression_budgets.json` defines release-scorecard benchmark regression tolerance.
- `policies/runtime_boundedness_budgets.json` defines per-profile boundedness ceilings for RSS, queues, stale surfaces, recovery time, and adapter restarts.
- Temporary compatibility mirrors should be declared only if benchmark migration debt is reintroduced; there are no active benchmark mirror registries right now.

Runtime artifacts may still be emitted under `core/local/artifacts/**` or runtime-local scorecard state, but budget truth belongs to Validation.
