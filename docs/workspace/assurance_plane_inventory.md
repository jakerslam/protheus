# Assurance Plane Inventory

Owner: Kernel / Assurance
Status: wave-1 inventory
Updated: 2026-04-29
Covers: `ASSURANCE-001` through `ASSURANCE-004`

## Purpose

This inventory classifies the current proof and observation machinery into the Assurance Plane domains defined by `docs/workspace/assurance_plane_policy.md`.

The goal is not to move code yet. The goal is to make ownership visible before migration and enforcement begin.

The physical migration status is tracked in `docs/workspace/assurance_physical_domain_migration_status.md`. This inventory is the domain-classification baseline; the migration-status document records which classified definitions now have canonical homes under `validation/**` or `observability/**` and which old locations remain compatibility debt.

## Domain Legend

| Domain | Meaning |
|---|---|
| `validation` | Controlled checks: tests, eval definitions, benchmarks, conformance guards, replay suites, regression suites. |
| `observability` | Live system awareness: telemetry, health, runtime traces, Sentinel evidence, source freshness, findings. |
| `governance` | Confidence and verdicts: release gates, scorecards, readiness verdicts, issue-candidate thresholds, promotion/retirement state. |
| `harness_only` | Execution wrapper, fixture host, or CI launcher that supports Assurance but is not itself source of truth. |

## ASSURANCE-001 Controlled Check Inventory

| Inventory item | Current source | Domain | Notes |
|---|---|---|---|
| Tooling gate registry | `tests/tooling/config/tooling_gate_registry.json` | validation | Registry-backed guard execution surface. Some entries currently expose only tooling metadata and need domain classification in a future schema. |
| Package test scripts | `package.json` scripts beginning with `test:*` | validation | Controlled TypeScript/Vitest/client-memory suites. |
| Package guard/gate scripts | `package.json` scripts containing `guard`, `gate`, `conformance`, `proof`, or `regression` | validation | Conformance and regression checks; later waves should register lifecycle and signal class. |
| Rust unit and integration tests | `core/**/tests`, crate-local Rust `#[test]` modules | validation | Controlled Kernel/Core proof. Harness remains Rust test runner; evidence should map into Validation envelopes when promoted. |
| Orchestration eval binaries | `orchestration` eval binaries invoked by `ops:eval:*` scripts | validation | Eval definitions and scoring belong to Validation even when implementation currently lives near Orchestration. Orchestration may consume results, not own eval truth. |
| Eval policy/config files | `tests/tooling/config/eval_*.json`, `tests/tooling/fixtures/eval_*`, `tests/tooling/schemas/eval_*` | validation | Gold datasets, adversarial matrices, quality thresholds, issue taxonomies, and schema definitions. |
| Release proof-pack assembly | `ops:release:proof-pack`, `tests/tooling/scripts/ci/release_proof_pack_assemble.ts` | validation | Controlled artifact assembly. Governance consumes the result for release posture. |
| Runtime proof gates | `ops:runtime-proof:*`, `ops:v8:runtime-proof:gate`, `TODO_RUNTIME_PROOF_BACKLOG.md` | validation | Controlled proof families that should emit Assurance evidence envelopes. |
| Layer conformance checks | `ops:layer2:*`, `ops:layer3:*`, `ops:arch:*`, `ops:nexus:*` | validation | Boundary and layer conformance proof. Some outputs also feed Governance when release-blocking. |
| Shell projection/detail guards | `ops:shell:*:guard`, shell UI contract guards | validation | Controlled checks proving Shell remains projection-only. Shell may not own Assurance truth. |
| Gateway/conduit guards | `ops:gateway:*`, `ops:conduit:*`, gateway manifest guards | validation | Controlled checks for external-boundary behavior. Gateway exposes routes but should not decide Assurance verdicts. |
| Public benchmark harness | `benchmarks/public_harness/run_public_harness.ts`, `ops:benchmark:public-harness` | validation | Benchmark definition and runner. Governance consumes budgets/trends. |
| Client-memory gate tests | `tests/client-memory-tools/*guard*.test.ts`, `*gate*.test.ts`, `*benchmark*.test.ts`, `*score*.test.ts` | validation | Large existing controlled-check surface. Future work should separate persistent self-enforcing proofs from temporary scaffolding. |

## ASSURANCE-002 Live Observation Producer Inventory

| Producer or stream | Current source | Domain | Notes |
|---|---|---|---|
| Sentinel evidence streams | `local/state/kernel_sentinel/evidence/*.jsonl` | observability | Canonical live evidence input surface for Sentinel. |
| Kernel receipts | `local/state/ops/verity/**` -> `kernel_receipts.jsonl` | observability | Deterministic Kernel authority evidence. |
| Runtime observations | `local/state/ops/system_health_audit/**` and runtime artifacts -> `runtime_observations.jsonl` | observability | Health, degraded/critical status, workflow failures, and runtime correctness evidence. |
| State mutation evidence | `core/local/artifacts/*stateful*`, `*rollback*`, `*upgrade*` -> `state_mutations.jsonl` | observability | Mutation legality and rollback evidence. |
| Scheduler/admission evidence | `core/local/artifacts/*scheduler*`, `*schedule*`, `*admission*`, layer contract artifacts -> `scheduler_admission.jsonl` | observability | Admission, capability, and fail-closed runtime evidence. |
| Live recovery evidence | `core/local/artifacts/*recovery*`, `*auto_heal*`, `*retry*`, `*rollback*` -> `live_recovery.jsonl` | observability | Recovery, retry, rollback, and degraded-state evidence. |
| Boundedness evidence | `core/local/artifacts/*boundedness*` -> `boundedness_observations.jsonl` | observability | RSS, queue depth, stale surfaces, recovery time, and resource ceiling evidence. |
| Gateway health/quarantine/recovery/isolation | `core/local/artifacts/*gateway*` -> gateway evidence streams | observability | External-boundary live health and isolation behavior. |
| Queue/backpressure evidence | `core/local/artifacts/*queue*`, `*backpressure*` -> `queue_backpressure.jsonl` | observability | Queue pressure, shed/defer/quarantine behavior, and policy outcomes. |
| Control-plane eval observations | `local/state/ops/eval_agent_feedback/**`, `local/state/ops/eval_learning_loop/**` -> `control_plane_eval.jsonl` | observability | Advisory workflow-quality observations. These may feed Sentinel but do not become Kernel authority. |
| Synthetic-user chat harness outputs | `local/state/ops/synthetic_user_chat_harness/**` | observability | Mixed source: deterministic route failures become runtime observations; grader-only judgments become advisory eval evidence. |
| Shell telemetry | `local/state/ops/shell_telemetry/**`, `core/local/artifacts/*shell*`, `*dashboard*` -> `shell_telemetry.jsonl` | observability | Presentation-only context. Cannot open Sentinel findings by itself. |
| Sentinel reports and trend outputs | `local/state/kernel_sentinel/**` | observability | Self-study outputs, not recursive primary evidence inputs. |

## ASSURANCE-003 Domain Classification Summary

| Family | Classification | Evidence source |
|---|---|---|
| Tests and regression suites | `validation` | `package.json`, `tests/client-memory-tools/**`, Rust crate tests. |
| Evals and judge lanes | `validation` for definitions and scoring; `observability` for live feedback rows | `ops:eval:*`, `orchestration` eval binaries, `local/state/ops/eval_*`. |
| Benchmarks | `validation` for benchmark definitions; `governance` when consumed as budgets/trends | `benchmarks/**`, `ops:benchmark:*`. |
| Conformance guards | `validation` | `ops:*:guard`, `ops:*:gate`, tooling registry entries. |
| Release gates | `governance` for verdicts; `validation` for controlled proof that feeds them | `ops:release:*`, `ops:production-closure:*`, release proof pack artifacts. |
| Scorecards | `governance` | Release scorecard scripts and scorecard artifacts. |
| Sentinel evidence ingestion | `observability` | `observability/source_coverage/assurance_observability_registry.json`, `observability/traces/sentinel_trace_source_map.json`, `docs/workspace/kernel_sentinel_evidence_source_map.md`, `local/state/kernel_sentinel/evidence/*.jsonl`. |
| Sentinel issue/feedback synthesis | `observability` plus `governance` when producing issue candidates or readiness blockers | `local/state/kernel_sentinel/feedback_inbox.jsonl`, `top_system_holes_current.json`, `rsi_readiness_summary_current.json`. |
| Tooling registry runners | `harness_only` | `tooling:list`, `tooling:run`, `tooling:profile`. |
| CI wrappers and local launchers | `harness_only` | Package scripts whose primary purpose is invoking a registered check. |

## ASSURANCE-004 Misplaced Ownership Flags

These flags are not defects by themselves. They are migration targets for the next Assurance waves.

| Flag | Current shape | Risk | Follow-up |
|---|---|---|---|
| Eval runtime lives near Orchestration | Several `ops:eval:*` scripts run `orchestration` binaries. | Orchestration can look like planner plus judge if definitions, scoring, feedback routing, and consumption stay co-located. | Move eval definition/score ownership into Validation while preserving Orchestration as a consumer/trigger. |
| Release scorecard script is tooling-local | `ops:release:scorecard:gate` writes a scorecard from a TS CI script. | Scorecards can become manually curated summaries unless every row points back to evidence artifacts. | Add scorecard derivation guard in `ASSURANCE-027`. |
| Sentinel currently spans observation and governance outputs | Sentinel emits findings, issue candidates, readiness, verdict, trends, and top holes. | Sentinel can be mistaken for all Validation rather than privileged Observability plus governance feed. | Keep Sentinel as Observability resident; route verdict/scorecard authority through Governance contracts. |
| Shell guards are numerous and shell-adjacent | Many `ops:shell:*` guards validate projection behavior from tooling. | Shell could appear to own health/readiness if display contracts are not separated from Assurance truth. | Add Shell truth-leak guard for Assurance state in `ASSURANCE-028`. |
| Tooling registry has limited domain metadata | Current registry exposes execution metadata more than Assurance domain/lifecycle/signal class. | Hard to distinguish scaffolding from release-grade validation. | Extend registry in `ASSURANCE-009` through `ASSURANCE-012`. |
| Benchmarks and budgets are not yet unified with governance | Public benchmark harness and boundedness artifacts exist separately from release scorecards. | Performance regressions may be measured without becoming consistent governance evidence. | Map benchmark outputs into Assurance envelope and Governance budgets. |
| Test maturity already exists but is not yet tied to Assurance | `ops:test-maturity:registry:guard` exists as a maturity mechanism. | Temporary scaffolding cleanup may stay disconnected from Validation lifecycle. | Use it as input to the Validation lifecycle registry. |

## Physical Migration Debt Markers

The current migration uses explicit debt markers instead of silent scattered ownership:

- compatibility mirrors under `validation/**/compatibility_mirrors.json`;
- the Observability mirror registry at `observability/compatibility_mirrors.json`;
- the time-bounded exemption registry at `validation/conformance/contracts/assurance_physical_domain_placement_exemptions.json`;
- `canonical_definition_paths` rows in `tests/tooling/config/tooling_gate_registry.json`;
- the placement guard `ops:assurance:physical-domain-placement:guard`.

Any future inventory item that is a definition, policy, registry, contract, or scoring rule should move into the owning Validation or Observability root unless it is explicitly marked `harness_only` or covered by a bounded compatibility exemption.

## Initial Migration Priority

1. Define the shared Assurance evidence envelope.
2. Extend the registry with `domain`, `signal_class`, `lifecycle_state`, and `evidence_output` fields.
3. Map Sentinel evidence rows and eval/proof-pack artifacts into the shared envelope.
4. Add scorecard derivation checks so scorecards remain summaries of evidence.
5. Add Shell and Orchestration ownership guards so consumers cannot become Assurance owners.
