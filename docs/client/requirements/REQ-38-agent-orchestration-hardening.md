# REQ-38: Agent Orchestration Hardening and Multi-Agent Audit Patterns

Version: 1.0
Date: 2026-03-15
Owner: Protheus Kernel / Cognition

## Objective

Harden multi-agent orchestration patterns to enable reliable parallel audits, coordinated task execution, and deterministic aggregation of sub-agent findings. Addresses gaps observed during SRS regression audits where parallel agents produced overlapping findings, timed out without recovery, and lacked standardized output formats.

## Scope

In scope:
- Coordinator agent pattern for partitioning work and deduplicating findings
- Shared state / scratchpad for cross-agent communication
- Checkpointing and timeout recovery mechanisms
- Strict scope boundaries to prevent overlap
- Standardized output schema for agent findings
- Completion triggers and task group metadata
- Partial result retrieval from timed-out agents

Out of scope:
- Replacing REQ-12 (Swarm Engine Router) — this extends it
- Replacing REQ-15 (Sandboxed Sub-Agent Execution) — this complements it
- Changing underlying spawn/session mechanics

## Related Requirements

- REQ-12: Swarm Engine Router (message routing, queue handoff)
- REQ-15: Sandboxed Sub-Agent Execution (isolated execution, scoped permissions)
- REQ-36: Smart Memory Low-Burn Regression Contract (shared state patterns)
- V6-SWARM-033 through V6-SWARM-038: spawned-agent tool manifests, hierarchical budgets, dead-letter recovery, restart recovery, the expanded dominance audit suite, and the generic-agent bootstrap contract for direct swarm bridge discovery

## Requirements

### REQ-38-001: Coordinator Agent Pattern

**Requirement:** Implement a coordinator agent that partitions work, deduplicates findings, and aggregates outputs from multiple sub-agents.

**Acceptance:**
- Coordinator accepts task description + list of subagent scopes
- Coordinator assigns non-overlapping work partitions to each sub-agent
- Coordinator receives findings from all sub-agents
- Coordinator deduplicates findings (same item_id reported by multiple agents)
- Coordinator merges severity ratings using highest-wins policy
- Coordinator emits unified report with consistent formatting

**Evidence:**
- Coordinator implementation in `client/cognition/orchestration/coordinator.ts`
- Test: `tests/client/cognition/coordinator.test.ts` (partitioning, dedupe, severity merge)
- Test: `tests/client/cognition/orchestration.integration.test.ts` (multi-agent integration)

---

### REQ-38-002: Shared State / Scratchpad

**Requirement:** Provide a shared workspace for cross-agent communication during multi-agent tasks.

**Acceptance:**
- Shared scratchpad file created at task start (`local/workspace/scratchpad/{task_id}.json`)
- Agents can read/write progress, findings, and checkpoints
- Scratchpad schema versioned for compatibility
- Scratchpad includes: items_checked[], findings[], progress_percent, last_updated
- Scratchpad is cleaned up after task completion (success or failure)

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs` and `core/layer0/ops/src/orchestration_parts/010-print-json-line.rs`
- Test: `core/layer0/ops/src/orchestration_parts/090-tests.rs` (corrupt scratchpad fails closed)
- Test: `core/layer0/ops/tests/orchestration_domain_integration.rs` (shared scratchpad lifecycle)

---

### REQ-38-003: Checkpointing and Timeout Recovery

**Requirement:** Agents must write progress to disk at intervals and support partial result retrieval on timeout.

**Acceptance:**
- Agents write checkpoint to scratchpad every 10 items or 2 minutes (whichever comes first)
- Checkpoint includes: last_item_id, findings_sofar[], timestamp
- On timeout, agent returns partial results + last checkpoint location
- Parent session can retrieve partial results via checkpoint path
- One automatic retry attempted before marking as failed

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs`, `core/layer0/ops/src/orchestration_parts/050-maybe-checkpoint.rs`, and `core/layer0/ops/src/orchestration_parts/060-retrieve-partial-results.rs`
- Test: `core/layer0/ops/src/orchestration_parts/090-tests.rs` (nonempty checkpoint fallback + timeout recovery)
- Test: `core/layer0/ops/tests/orchestration_domain_integration.rs` (coordinator partial recovery)

---

### REQ-38-004: Strict Scope Boundaries

**Requirement:** Enforce domain scoping via explicit allowlists to prevent overlapping work assignments.

**Acceptance:**
- Scope format supports: `series:[V3-SEC,V4-SEC]` or `paths:[adapters/cognition/*]`
- Coordinator validates scope non-overlap before spawning
- Agents report out-of-scope findings separately (not as primary findings)
- Scope violations logged to coordinator for reassignment

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs` and `core/layer0/ops/src/orchestration_parts/030-detect-scope-overlaps.rs`
- Test: `core/layer0/ops/src/orchestration_parts/090-tests.rs` (duplicate `scope_id` rejection)
- Test: `core/layer0/ops/tests/v9_swarm_runtime_integration_parts/050-orchestration-hardening-tests.rs` (swarm-facing scope hardening)

---

### REQ-38-005: Standardized Output Schema

**Requirement:** All agents return findings in a consistent JSON structure.

**Acceptance:**
- Schema defined in `client/cognition/orchestration/schemas/finding-v1.json`
- Required fields: audit_id, item_id, severity, status, location, evidence, timestamp
- Severity enum: CRITICAL, HIGH, MEDIUM, LOW
- Status enum: missing, partial, drift, compliant
- Location format: "file:line" or "file:line:column"
- Schema validation enforced before accepting agent results

**Evidence:**
- Enforcement in `core/layer0/ops/src/orchestration.rs` and `core/layer0/ops/src/orchestration_parts/020-validate-finding.rs`
- Test: `core/layer0/ops/src/orchestration_parts/090-tests.rs` (invalid severity rejection)
- Test: `core/layer0/ops/tests/v9_swarm_runtime_integration_parts/050-orchestration-hardening-tests.rs` (deduped retry payloads)

---

### REQ-38-006: Completion Triggers

**Requirement:** Auto-notify parent session when all subagents in a task group complete.

**Acceptance:**
- Task group ID assigned at spawn time (`task_group: srs-audit-2026-03-15`)
- System tracks agent status: pending, running, done, failed, timeout
- When all agents report done/failed/timeout, parent session notified
- Notification includes: completed_count, failed_count, timeout_count, partial_count
- Optional: auto-aggregate results into unified report

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs` and `core/layer0/ops/src/orchestration_parts/070-run-coordinator.rs`
- Test: `core/layer0/ops/tests/orchestration_domain_integration.rs` (aggregate completion details)

---

### REQ-38-007: Task Group Metadata

**Requirement:** Tag subagents with task group ID for collective tracking and querying.

**Acceptance:**
- Task group ID format: `{task_type}-{timestamp}-{nonce}`
- All subagents in group tagged with group ID in session metadata
- API supports querying all agents by task group ID
- Task group metadata includes: created_at, coordinator_session, agent_count, status

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs`
- Test: `core/layer0/ops/tests/orchestration_domain_integration.rs` (task-group metadata and group queries)

---

### REQ-38-008: Partial Result Retrieval

**Requirement:** Ability to fetch partial results from timed-out or failed agents.

**Acceptance:**
- API: `sessions_history(sessionKey, includeTools=true)` returns partial results
- Fallback: Read checkpoint files from workspace if session unavailable
- Partial results include: items_completed, findings_sofar[], checkpoint_path
- Parent session can decide: retry, continue with partial, or abort

**Evidence:**
- Implementation in `core/layer0/ops/src/orchestration.rs` and `core/layer0/ops/src/orchestration_parts/060-retrieve-partial-results.rs`
- Test: `core/layer0/ops/src/orchestration_parts/090-tests.rs` (task-group and checkpoint fallback retrieval)
- Test: `core/layer0/ops/tests/orchestration_domain_integration.rs` (parent decision flow)

## Verification Requirements

- Unit tests for each REQ-38-00X component
- Integration test: Full multi-agent audit with coordinator, scratchpad, and completion triggers
- Load test: 20+ parallel agents with checkpointing and timeout recovery
- Invariant: No overlapping work assignments in partitioned tasks
- Invariant: All findings conform to standardized schema
- Swarm-runtime hardening must also preserve:
  - authoritative spawned-agent tool manifests (`sessions_send`/`sessions_query`/`sessions_state` exposed from spawn receipts, not inferred),
  - hierarchical token reservation/settlement across parent-child chains,
  - dead-letter + retry recovery under TTL expiry/backpressure,
  - persistent-session resume after runtime reload.

## Execution Notes

- Priority order: REQ-38-005 (schema) → REQ-38-002 (scratchpad) → REQ-38-006 (completion) → REQ-38-001 (coordinator) → REQ-38-003 (checkpointing) → REQ-38-004 (scope) → REQ-38-007 (task group) → REQ-38-008 (partial results)
- Start with schema and shared state — everything else builds on these
- Coordinator can be lightweight initially — focus on correct partitioning over sophisticated algorithms
- Checkpointing is critical for long-running audits (SRS regression took 4-6 minutes per agent)

## Amendment Notes

- REQ-15-002 (Dynamic sub-agent spawning) should reference REQ-38-007 for task group metadata
- REQ-12-008 (Swarm observability receipts) should reference REQ-38-005 for standardized finding schema
- REQ-36 (Smart Memory) scratchpad implementation may share patterns with REQ-38-002

## Companion Evidence Updates

### 2026-03-19: Swarm Runtime Companion Update

- Repair-lane execution now routes by contract runtime (`srs_contract_runtime` vs `runtime_systems`) to prevent stale path failures and ensure deterministic contract receipts for swarm-adjacent lanes.
- Ranked ROI lane execution was expanded and verified at 300-lane scale with deterministic lane-only execution controls.
- Current evidence references:
  - `core/layer0/ops/src/swarm_runtime.rs`
  - `tests/tooling/scripts/ci/srs_repair_lane_runner.ts`
  - `tests/tooling/scripts/ci/roi100_moves_runner.ts`
  - `core/layer0/ops/tests/v6_infring_closure_integration.rs`

### 2026-04-10: Lineage Messaging + Tool Attempt Transparency Update

- Lineage-adjacent message delivery is now explicitly allowed for parent/child and sibling session pairs without granting full manage-agent authority; destructive lifecycle actions remain descendant-scoped.
- Direct tool routes now preserve structured `tool_attempt_receipt` payloads even for blocked/unavailable/error outcomes so parent coordinators can tell whether a child actually attempted a tool call.
- Current evidence references:
  - `core/layer0/ops/src/swarm_runtime_parts/050-verify-session-reachable.rs`
  - `core/layer0/ops/src/swarm_runtime_parts/171-lineage-message-tests.rs`
  - `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/010-prelude-and-session.rs`
  - `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/020-message-direct-tool.rs`
  - `core/layer0/ops/src/dashboard_compat_api_parts/config_payload_tests_parts/040-terminated-agent-endpoints-round-trip_parts/030-part.rs`

### 2026-04-11: Swarm Audit + Companion Surface Update

- Swarm dominance auditing now covers 14 cases, adding workflow receipt compaction/handoff coverage and parent-child directive delivery on top of concurrency, budgets, dead letters, restart recovery, and generic-agent bootstrap checks.
- Companion swarm proofs now cover Rust/runtime parity, thorn quarantine/release, RSI swarm gate failure semantics, verification-mode receipt surfacing, and the mobile-edge swarm dispatch contract so stale legacy wrappers no longer stand in for live swarm evidence.
- Current evidence references:
  - `tests/tooling/scripts/ci/swarm_protocol_audit_runner.ts`
  - `tests/client-memory-tools/swarm_workflow_007_bridge.test.ts`
  - `tests/client-memory-tools/swarm_runtime_smoothness.test.ts`
  - `tests/client-memory-tools/swarm_phase7_rust_parity.test.ts`
  - `tests/client-memory-tools/thorn_swarm_protocol.test.ts`
  - `tests/client-memory-tools/rsi_swarm_spawn_bridge.test.ts`
  - `tests/client-memory-tools/swarm_verification_mode.test.ts`
  - `tests/client-memory-tools/mobile_edge_swarm_bridge.test.ts`
  - `core/layer0/ops/src/protheusctl_parts/020-evaluate-dispatch-security.rs`
