# Benchmark Metric Classification

## Purpose

Separate public benchmark interpretation into non-overlapping metric classes so readiness and throughput claims remain auditable.

## Classes

1. `readiness`
- Source fields: `cold_start_ms`, `kernel_ready_ms`, `gateway_ready_ms`, `dashboard_interactive_ms`.
- Meaning: status-path readiness and startup-stage timing indicators.
- Non-goal: not a full stopped-from-zero system boot metric.

2. `kernel_shared_throughput`
- Source field: `kernel_shared_workload_ops_per_sec`.
- Meaning: synthetic/shared workload throughput for kernel-level comparison.
- Non-goal: not user-facing command throughput.

3. `end_to_end_command_throughput`
- Source field: `rich_end_to_end_command_path_ops_per_sec`.
- Meaning: governed rich command-path throughput under operator-facing flow.
- Non-goal: not directly comparable to synthetic shared-workload throughput.

## Reporting Requirements

1. README must keep class-separation language and caveats explicit.
2. Public benchmark snapshots must preserve field names exactly as emitted in benchmark artifacts.
3. Release proof packs must include the benchmark artifact used for published summary claims.

## Operator Commands

- Refresh benchmark artifact:
  - `npm run -s ops:benchmark:refresh`
- Sanity check benchmark artifact:
  - `npm run -s ops:benchmark:sanity`
- Verify public benchmark contract:
  - `npm run -s ops:benchmark:public-audit`

