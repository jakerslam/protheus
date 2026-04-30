# System Map

Generated: 2026-04-30T14:58:38.545Z

This map is generated from `client/runtime/config/system_map_registry.json` via `system_map_generator` and is the canonical quick-reference for subsystem purpose, ownership, and health checks.

## Layer Coverage

| Layer | Subsystems |
|---|---:|
| Conduit | 1 |
| Gateway | 1 |
| Governance | 1 |
| Layer -1 | 1 |
| Layer 0 | 5 |
| Layer 2 | 1 |
| Layer 3 | 1 |
| Observability | 2 |
| Orchestration | 1 |
| Shell | 1 |
| Shell Cognition | 10 |
| Shell Ops | 2 |
| Validation | 1 |

## Subsystem Map

| Subsystem | Layer | Purpose | Owner | Inputs | Outputs | Failure Mode | Health Check | SRS |
|---|---|---|---|---|---|---|---|---|
| Exotic Wrapper | Layer -1 | Normalizes exotic substrate signals into deterministic Layer 0 envelopes. | kernel | substrate probes; adapter capability metadata | layer0 envelope; degradation contract | unsupported_substrate_profile | `cargo check -p exotic_wrapper` | `V6-ARCH-LAYERING-031` |
| Origin Integrity | Layer 0 | Enforces infallible-origin checks before autonomous runtime actions. | kernel | startup self-audit; manual origin verify | origin integrity receipt; degraded/fail-closed decision | origin_integrity_timeout_or_mismatch | `infring-ops origin-integrity status` | `V6-ORIGIN-001`, `V6-ORIGIN-004` |
| Global Importance Kernel | Layer 0 | Scores cross-system events and assigns deterministic priority/initiative actions. | kernel | attention queue event; severity and context metadata | score; band; priority; initiative_action | layer2_priority_authority_unavailable | `npm run -s ops:test:infring-ops-core:attention` | `V6-INITIATIVE-013` |
| Attention Queue | Layer 0 | Maintains TTL/dedupe/backpressure-governed priority event queue. | kernel | eyes; memory auto-recall; dopamine; spine; shadow dispatch | priority-ordered events; cursor/ack receipts | queue_backpressure_or_gate_active | `infring-ops attention-queue status` | `V6-COCKPIT-002`, `V6-COCKPIT-006`, `V6-INITIATIVE-013` |
| Swarm Router | Layer 0 | Provides typed task routing with retry, scaling, and queue contracts. | kernel | coordinator tasks; worker status | route decisions; in-flight metrics; recovery receipts | cargo_test_timeout_host_stall | `cargo test -p swarm_router` | `V6-SWARM-001`, `V6-SWARM-006` |
| Adaptive Runtime (Kernel) | Layer 0 | Owns adaptation cadence/resource contracts and emits deterministic adaptation receipts. | kernel | adaptive tick; runtime metrics | adaptation receipt; status snapshot | adaptive_policy_guard_denied | `infring-ops adaptive-runtime status` | `V6-ADAPT-CORE-001` |
| Initiative Primitives | Layer 2 | Provides deterministic initiative score/action primitives for cockpit-safe proactive behavior. | kernel | importance payload; attention metadata | initiative action; priority shaping | layer2_execution_lane_unavailable | `npm run -s ops:test:execution-core:initiative` | `V6-ARCH-ICEBERG-028`, `V6-INITIATIVE-013` |
| Conduit Bridge | Conduit | Enforces conduit-only communication between shell surfaces and Kernel authority. | kernel+shell | shell lane request; daemon probes | typed conduit response; runtime gate diagnostics | conduit_stdio_timeout_or_runtime_gate | `node client/lib/conduit_full_lifecycle_probe.ts` | `V6-COCKPIT-005`, `V6-CONDUIT-RUNTIME-STALL-001` |
| Orchestration Control Plane | Orchestration | Coordinates intake, decomposition, workflow selection, recovery, and result shaping without owning Kernel truth. | orchestration | typed user request; probe envelope; execution observations | plan candidates; decision traces; result package | planner_truth_or_probe_authority_gap | `cargo check --manifest-path orchestration/Cargo.toml` | `V13-ASSURANCE-PHYSICAL-DOMAINS-001`, `V13-ARCH-POLICY-GOVERNANCE-001` |
| Gateway Membrane | Gateway | Converts external ambiguity into bounded, leased, auditable ingress, egress, health, detail, and search routes. | gateway | shell requests; CLI/SDK requests; external agent traffic | bounded projections; detail refs; audit receipts | gateway_route_unbounded_or_authority_mirror | `npm run -s ops:gateway:interface:guard` | `V13-ARCH-GATEWAY-INTERFACE-001` |
| Shell Projection Surface | Shell | Collects operator input and renders bounded projections without mirroring runtime authority or full state. | shell | gateway projection rows; detail refs; operator input | display state; bounded requests; operator actions | shell_projection_becomes_state_mirror | `npm run -s ops:shell:projection:guard` | `V13-ARCH-SHELL-INDEPENDENCE-001`, `V13-ARCH-SHELL-INDEPENDENCE-003` |
| Validation Domain | Validation | Owns controlled tests, evals, benchmarks, conformance checks, regression suites, release gates, fixtures, and proof artifacts. | validation | controlled check definitions; fixtures; release evidence | pass/fail evidence; regression artifacts; scorecard inputs | validation_definition_outside_domain | `npm run -s ops:assurance:placement:guard` | `V13-ASSURANCE-PHYSICAL-DOMAINS-001` |
| Observability Domain | Observability | Owns live telemetry, health, universal traces, Sentinel evidence streams, runtime findings, freshness, and source coverage. | observability | runtime telemetry; trace spans; source health rows | live evidence envelopes; trace stories; runtime findings | fragmented_observability_or_stale_source_coverage | `npm run -s ops:assurance:evidence-envelope:guard` | `V13-ASSURANCE-PHYSICAL-DOMAINS-001` |
| Kernel Sentinel | Observability | Watches Kernel, Gateway, Orchestration, Shell, and runtime evidence to synthesize failures, architectural incidents, issue candidates, and RSI blockers. | observability+kernel | runtime evidence; guard artifacts; diagnostic results | findings; architectural incidents; issue candidates; self-study reports | sentinel_evidence_undercoverage_or_symptom_fixation | `cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel -- --nocapture` | `V13-ASSURANCE-PHYSICAL-DOMAINS-001` |
| Governance Domain | Governance | Derives readiness verdicts, scorecards, gates, issue-candidate routing, and release decisions from Validation and Observability evidence. | governance | validation evidence; observability evidence; trend deltas | verdicts; scorecards; release blockers; issue candidates | governance_verdict_without_evidence | `npm run -s ops:assurance:scorecard-derivation:guard` | `V13-ASSURANCE-PHYSICAL-DOMAINS-001` |
| Layer 3 OS Personality Template | Layer 3 | Defines the future process, VFS, driver, syscall, namespace, networking, and windowing substrate without becoming a junk drawer. | kernel | Layer 2 execution intents; resource budgets; namespace requests | process/service contracts; resource envelopes; namespace handles | layer3_contract_violation_or_unowned_os_feature | `rg -n "Layer 3" ARCHITECTURE.md` | `V13-ARCH-NEXUS-FEDERATION-001` |
| Persistent Cockpit Daemon | Shell Ops | Attach-first daemon that keeps ambient loop, subscribe lane, and resident state alive across sessions. | shell | attach/start/subscribe commands; attention queue drain | status envelopes; degraded class diagnostics; subscribe batches | bridge_degraded_or_origin_pending | `node client/runtime/systems/ops/infringd.ts status` | `V6-COCKPIT-001`, `V6-COCKPIT-004`, `V6-COCKPIT-005` |
| Cockpit Harness | Shell Ops | Surfaces critical ambient alerts and cockpit-ready telemetry for mech-suit operation. | shell | daemon status; alert events | cockpit alert artifacts; harness receipts | alert_publish_failed | `node client/runtime/systems/ops/cockpit_harness.ts status` | `V6-MECH-014` |
| Eyes/Sensory Intake | Shell Cognition | Collects external signals and routes governed events to memory/attention lanes. | shell | collectors; eye directives | signal events; memory bridge entries | collector_failure_or_route_denied | `node client/runtime/systems/sensory/eyes_intake.ts status` | `V6-SHADOW-003` |
| Conversation Eye | Shell Cognition | Synthesizes dialogue into tagged memory nodes and forwards recall triggers. | shell | session messages | conversation nodes; auto-recall triggers | synthesis_write_failed | `npm run -s test:ops:conversation-eye-collector` | `V6-COGNITION-010` |
| Memory Matrix | Shell Cognition | Maintains scored tag-to-memory index for low-burn retrieval and ranking. | shell | daily nodes; conversation eye; dream flags | ranked matrix entries; tag coverage | matrix_rebuild_failed | `npm run -s test:memory:matrix` | `V6-MEMORY-011` |
| Dream Sequencer | Shell Cognition | Periodically reorders memory relevance and refreshes top-priority recall surfaces. | shell | memory matrix | reordered priorities; dream sequence receipts | dream_cycle_degraded | `npm run -s memory:dream-sequencer:status` | `V6-MEMORY-011` |
| Memory Auto-Recall | Shell Cognition | Pushes bounded high-overlap memory matches into attention without full-file scans. | shell | new memory node; tag overlap matrix | attention queue enqueue | attention_enqueue_failed | `npm run -s test:memory:auto-recall` | `V6-MEMORY-011` |
| Shadow Signal Classifier | Shell Cognition | Classifies sensory signals into shadow routes with confidence and reason receipts. | shell | eye signals | shadow route map; classifier receipt | no_route_match | `npm run -s test:lane:run -- --id=V6-SHADOW-003` | `V6-SHADOW-003` |
| Shadow Dispatch Reliability | Shell Cognition | Provides idempotent enqueue/retry/ack dispatch contract for routed shadow tasks. | shell | classifier route; dispatch request | dispatch queue state; escalation/ack receipts | dispatch_queue_stall | `npm run -s test:lane:run -- --id=V6-SHADOW-004` | `V6-SHADOW-004` |
| Browser Text/Diff Lane | Shell Cognition | Emits token-efficient browser text snapshots and compact diffs instead of heavy payloads. | shell | html/text snapshot; before/after text | text snapshot receipt; diff receipt with token reduction | snapshot_parse_failed | `npm run -s test:lane:run -- --id=V6-BROWSER-007` | `V6-BROWSER-007` |
| Realtime Adaptation Loop | Shell Cognition | Applies interaction-driven adaptation under drift/covenant gates with continuity checks. | shell | interaction/heartbeat triggers; resource metrics | adaptation cycle receipts; review bridge submissions | cadence_throttle_or_drift_gate | `npm run -s test:lane:run -- --id=V6-ADAPT-004` | `V6-ADAPT-004`, `V6-ADAPT-005`, `V6-ADAPT-006` |
| Low-Burn Reflexes | Shell Cognition | Provides capped helper reflexes for common tasks without large token overhead. | shell | reflex request | bounded reflex response | token_cap_violation | `npm run -s test:reflexes` | `V6-REFLEX-CORE-001` |
