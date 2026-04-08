# Public Benchmarks

Canonical public benchmark artifact (repo-tracked):

- [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

Stabilized comparison baselines (repo-tracked):

- [`docs/client/reports/benchmark_matrix_stabilized_2026-03-19.json`](docs/client/reports/benchmark_matrix_stabilized_2026-03-19.json)
- [`docs/client/reports/benchmark_matrix_stabilized_preflight_2026-03-20.json`](docs/client/reports/benchmark_matrix_stabilized_preflight_2026-03-20.json)

Reproduce and validate:

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
npm run -s ops:benchmark:public-audit
npm run -s ops:benchmark:repro
```
