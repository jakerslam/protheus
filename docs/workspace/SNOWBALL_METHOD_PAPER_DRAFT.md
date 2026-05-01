# Snowball Method: Evidence-First Compaction and Deployment for an Agentic OS

Draft date: 2026-03-15  
Status: Working draft for operator review

## Abstract

This draft describes the Snowball Method as applied to InfRing/Infring: an iterative, evidence-first cycle that compounds architecture quality while preserving strict governance boundaries (Rust-core authority, thin-client surface, deterministic receipts, and fail-closed policy checks). The latest cycle demonstrates an ultra-compact Tiny-max runtime profile with sub-300KB daemon size, low cold-start latency, and high throughput while maintaining the same security system depth as richer operating modes.

## 1. Method

The Snowball cycle used in this repository follows a strict loop:

1. Ingest constraints from SRS and operator directives.
2. Execute smallest high-ROI tranche with deterministic receipts.
3. Validate behavior with regression + integration + CLI evidence.
4. Compact duplicated surfaces into canonical primitives.
5. Publish measured benchmark artifacts and update public tables.
6. Carry unresolved external blockers as explicit human-owned items.

This prevents "status theater" and keeps public claims bound to reproducible outputs.

## 2. Current Measured Results

Measured from the current benchmark lane and artifact set:

| Mode | Install Size (MB) | Cold Start (ms) | Idle Memory (MB) | Throughput (ops/sec) | Static Daemon (MB) |
|---|---:|---:|---:|---:|---:|
| Infring (rich) | 9.920 | 6.442 | 9.844 | 11,109 | 0.460 |
| Pure Workspace | 0.671 | 4.072 | 1.375 | 11,131 | 0.460 |
| Pure Workspace Tiny-max | 0.483 | 3.708 | 1.375 | 10,987 | 0.263 |

## 3. Competitive Position

Against public baseline values in Infring/OpenHands tables, the Tiny-max profile prioritizes small footprint and fast startup while keeping a high security-system count in the same control plane. The competitive delta is largest on install footprint and daemon size.

## 4. Evidence and Reproducibility

Primary evidence sources are repo-local, deterministic, and refreshable:

1. `npm run -s ops:benchmark:refresh` regenerates benchmark matrices.
2. Tiny-max daemon size is verified via `stat` on `infringd_tiny_max`.
3. README benchmark section is updated only from these artifacts.

## 5. External Blocker and Next Experiment

A real hardware MCU proof pass (ESP32 + RP2040 flash + runtime screenshot evidence) remains externally blocked on physical board access and human-operated flash sessions.

- TODO reference: `P0-MCU-PROOF-001` (`docs/workspace/TODO.md`)
- Human ownership reference: `HMAN-092` (`docs/client/HUMAN_ONLY_ACTIONS.md`)
- Operator runbook: `docs/ops/RUNBOOK-005-mcu-proof-sprint.md`

When these artifacts are attached, this draft can be upgraded to a publication-ready technical report with figure panels from real hardware runs.

## References

1. Benchmark matrix (live): `docs/client/reports/benchmark_matrix_run_2026-03-06.json`
2. Benchmark matrix (full-install): `docs/client/reports/benchmark_matrix_run_2026-03-06_full_install.json`
3. Runtime snapshot baseline: `docs/client/reports/runtime_snapshots/ops/proof_pack/top1_benchmark_snapshot.json`
4. Public comparative baseline (Infring): https://raw.githubusercontent.com/RightNow-AI/infring/main/README.md
5. Snowball app contract surface: `apps/snowball_engine/README.md`
