# Competitive Benchmark Matrix (REQ-13-002)

Deterministic benchmark entrypoint for competitive parity claims.

## Metrics

- `cold_start_ms`
- `idle_memory_mb`
- `install_size_mb`
- `evidence_verify_latency_ms`

## Run

```bash
./benchmarks/competitive_matrix/run_matrix.sh
```

The runner writes runtime receipts into the local runtime state directory (not committed), and
publishes the canonical public snapshot as a repo-tracked artifact:

- `docs/client/reports/benchmark_matrix_run_latest.json`

For externally auditable publication (tracked artifact refresh + sanity):

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
npm run -s ops:benchmark:public-audit
# one-shot reproducibility lane:
npm run -s ops:benchmark:repro
```
