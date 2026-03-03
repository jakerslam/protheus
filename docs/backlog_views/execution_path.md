# Backlog Execution Path

Generated: 2026-03-03T01:41:27.217Z

## Summary

- Queued rows: 279
- Lane run commands discovered: 34
- Lane coverage (queued rows with lane): 5.73%
- Runnable now (lane + deps closed): 4
- Runnable but blocked by deps: 12
- Ready but no lane implementation: 33
- Blocked + no lane implementation: 230

## Recommended Next Actions

- Execute 4 runnable rows with existing lane commands first (lane:<id>:run + corresponding test:lane:<id>).
- For 33 dependency-ready rows without lanes, add runtime lane + test artifacts before marking done.
- Prioritize blocker dependencies (V3-RACE-214:21, V3-RACE-174:16, V3-RACE-019:14, V3-RACE-209:13, V3-RACE-210:12, V3-RACE-218:10, V3-RACE-129:9, V3-RACE-211:9, V4-SCALE-001:9, V3-RACE-044:8) to unlock blocked rows fastest.

## Runnable Now

| ID | Wave | Class | Lane | Open Dependencies | Title |
|---|---|---|---|---|---|
| V3-RACE-DEF-025 | V3 | hardening | yes |  | Smart Knot Crown-Jewel Obfuscation Layer |
| V3-RACE-CONF-007 | V3 | hardening | yes |  | Permanent Guidelines Drift Gate + Ticket Output Contract |
| V3-RACE-051 | V3 | hardening | yes |  | Hardware Root-of-Trust Attestation Mesh |
| V3-RACE-052 | V3 | hardening | yes |  | Data Poisoning Immunity + Causal Rollback Plane |

## Ready But Missing Lane Implementation

| ID | Wave | Class | Lane | Open Dependencies | Title |
|---|---|---|---|---|---|
| V4-RUST-002 | V4 | scientific | no |  | Scientific Stack Rust Orchestration + R Bridge Hardening |
| V4-RUST-004 | V4 | extension | no |  | Habits/Vault/Adaptive Core Rust Migration Wave (Phased) |
| V3-RACE-025 | V3 | hardening | no |  | Daemon Supervision + Stale PID/Socket Reaper |
| V3-RACE-027 | V3 | primitive-upgrade | no |  | Direct Memory Encryption Plane Integration (Replace DB Shim) |
| V3-RACE-001 | V3 | primitive-upgrade | no |  | Rust Core Runtime Kernel Slice |
| V3-RACE-003 | V3 | primitive-upgrade | no |  | Event-Sourced Control Plane + CQRS Materialized Views |
| V3-RACE-004 | V3 | primitive-upgrade | no |  | Model Catalog + Live Routing Scoreboard Service |
| V3-RACE-005 | V3 | hardening | no |  | Thought-to-Action Trace Contract (Intent -> Model -> Tool -> Outcome) |
| V3-RACE-006 | V3 | primitive | no |  | Swarm Orchestration Runtime (Hierarchical + Election + Consensus) |
| V3-RACE-007 | V3 | extension | no |  | Cross-Cell Memory Exchange Plane (Policy-Governed) |
| V3-RACE-008 | V3 | primitive-upgrade | no |  | Sovereign Personality Substrate ("Soul Vector") |
| V3-RACE-009 | V3 | primitive-upgrade | no |  | Hybrid Memory Engine (Vector + Graph + Temporal) |
| V3-RACE-010 | V3 | extension | no |  | Memory Consolidation + Forgetting Curves |
| V3-RACE-012 | V3 | hardening | no |  | Observability Deployment Defaults (Prometheus/Grafana/Loki/Trace) |
| V3-RACE-013 | V3 | extension | no |  | Compatibility Spec + Conformance Badge Program |
| V3-RACE-CONF-001 | V3 | extension | no |  | Open Platform Path-Contract Compatibility Pack (`platform/` artifacts) |
| V3-RACE-CONF-003 | V3 | hardening | no |  | Requirement Conformance Matrix + Gate (`external prompt -> canonical lane`) |
| V3-RACE-CONF-004 | V3 | hardening | no |  | Rust Memory Path-Contract Compatibility (`core/memory` alias docs/wrappers) |
| V3-RACE-CONF-005 | V3 | hardening | no |  | N-API Build Surface Compatibility Contract (`build:memory`/postinstall expectations) |
| V3-RACE-037 | V3 | extension | no |  | Long-Term Archival & Sovereign Resurrection Substrate |
| V3-RACE-060 | V3 | hardening | no |  | Dist Runtime Contract Reconciliation Gate (Legacy-Pair Truth Source) |
| V3-RACE-068 | V3 | extension | no |  | Advisory JS Purge Wave (`habits/scripts`, `memory/tools`) |
| V3-RACE-070 | V3 | primitive-upgrade | no |  | Top-K Execution Reservation Lane |
| V3-RACE-071 | V3 | hardening | no |  | Filter Pressure Rebalancer (High-Score Exemption Contracts) |
| V3-RACE-072 | V3 | extension | no |  | Action-Spec Auto-Enrichment Lane |
| V3-RACE-073 | V3 | hardening | no |  | Queue Debt Backpressure + Intake Throttle Mode |
| V3-RACE-074 | V3 | hardening | no |  | Eye Health SLO + Auto-Heal Escalation Lane |
| V3-RACE-075 | V3 | hardening | no |  | Execution Floor Contract (Sunday Included, Explicit Observation Override) |
| V3-RACE-076 | V3 | extension | no |  | Execution-to-Artifact Auto-Capture Bridge |
| V3-RACE-119 | V3 | hardening | no |  | Unified CI Quality Scorecard & Gates |

## Top Dependency Blockers

| Dependency | Blocked Rows |
|---|---|
| V3-RACE-214 | 21 |
| V3-RACE-174 | 16 |
| V3-RACE-019 | 14 |
| V3-RACE-209 | 13 |
| V3-RACE-210 | 12 |
| V3-RACE-218 | 10 |
| V3-RACE-129 | 9 |
| V3-RACE-211 | 9 |
| V4-SCALE-001 | 9 |
| V3-RACE-044 | 8 |
| V3-RACE-130 | 8 |
| V3-RACE-031 | 7 |
| V3-RACE-137 | 7 |
| V3-RACE-200 | 6 |
| V3-RACE-212 | 6 |
| V3-RACE-223 | 6 |
| V3-RACE-245 | 6 |
| V4-SETTLE-001 | 6 |
| V3-RACE-037 | 5 |
| V3-RACE-041 | 5 |
| V3-RACE-201 | 5 |
| V3-RACE-249 | 5 |
| V3-RACE-250 | 5 |
| V3-RACE-269 | 5 |
| V3-RACE-280 | 5 |
| V3-RACE-022 | 4 |
| V3-RACE-161 | 4 |
| V3-RACE-165 | 4 |
| V3-RACE-172 | 4 |
| V3-RACE-203 | 4 |
| V3-RACE-222 | 4 |
| V3-RACE-229 | 4 |
| V3-RACE-239 | 4 |
| V3-RACE-258 | 4 |
| V3-RACE-263 | 4 |
| V3-RACE-270 | 4 |
| V3-RACE-283 | 4 |
| V3-RACE-304 | 4 |
| V3-RACE-309 | 4 |
| V3-RACE-315 | 4 |

