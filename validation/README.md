# Validation Domain

Owner: Assurance / Validation
Status: physical domain anchor

Validation owns controlled confidence checks: tests, evals, benchmarks, conformance guards, regression suites, release gates, scorecards, governance verdict contracts, fixtures, schemas, and reports.

Validation answers: "Does this system behave correctly under controlled checks?"

## Authority Boundary

Validation may define controlled checks, scoring rubrics, release-gate inputs, benchmark budgets, regression fixtures, scorecard derivation contracts, governance verdict contracts, and issue-candidate routing policies.

Validation must not own runtime planning, Kernel policy truth, Shell presentation state, live telemetry truth, or production state mutation.

## Subdomains

- `tests/` controlled test definitions and test-domain metadata.
- `evals/` eval definitions, rubrics, fixtures, and scoring contracts.
- `benchmarks/` benchmark definitions and performance budgets.
- `conformance/` architecture, boundary, schema, and policy conformance guards.
- `regression/` named regression suites and replay scenarios.
- `release_gates/` release-blocking controlled checks and promotion gates.
- `scorecards/` evidence-derived scorecard definitions and templates.
- `governance/` evidence-derived verdict inputs/outputs, promotion rules, and issue-candidate routing.
- `schemas/` Validation-owned schemas.
- `fixtures/` controlled fixtures used by Validation checks.
- `reports/` generated or latest Validation report destinations.

## Migration Rule

Existing commands may keep compatibility wrappers while migration is active, but new controlled-check definitions should land here unless explicitly marked `harness_only` or compatibility debt.
