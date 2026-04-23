# Client Runtime SRS Supplement (Authority Migration)

Updated: 2026-03-27 17:36 America/Denver
Owner: Runtime Coreization
Status: Active (supplemental until merged into `docs/workspace/SRS.md`)

## Purpose
This supplement captures client/runtime functional requirements that are not comprehensively codified in `SRS.md`, with explicit migration constraints for moving authority into Rust core while preserving behavior.

## Scope
- In scope: `client/runtime/**`, `client/cognition/**`, `client/lib/**` authority behavior.
- Out of scope: pure visual styling details (tracked in UI supplement).

## Global invariants
- `CSR-INV-001`: New authority must land in `core/**`.
- `CSR-INV-002`: Client surfaces must be thin wrappers, transport, rendering, or developer tooling only.
- `CSR-INV-003`: Every migration slice must preserve CLI/API compatibility and deterministic receipts.
- `CSR-INV-004`: No authored JavaScript may be introduced.

## A. Agent lifecycle + persistence
- `CSR-AGENT-001`: Agent creation must be deterministic and never fail silently.
- `CSR-AGENT-002`: Agent identity (`id`, `name`, `avatar`, `emoji`) must persist across dashboard/gateway restarts.
- `CSR-AGENT-003`: Main-session agents marked permanent/immortal must never auto-term.
- `CSR-AGENT-004`: TTL-based agents must term only when expiry policy says so; term reason must be explicit.
- `CSR-AGENT-005`: Timed-out agents must transition to archived/terminated state, never zombie state.
- `CSR-AGENT-006`: `agent_not_found` must trigger bounded recovery (resync + stale reference prune), not user-facing deadlock.
- `CSR-AGENT-007`: Delete/archive must remove runtime bindings and stale menu entries atomically.
- `CSR-AGENT-008`: If configured, branch cleanup on delete must run in the same transaction and produce receipt.

## B. Conversation isolation + integrity
- `CSR-CONV-001`: Messages must be partitioned by conversation id + agent id (no cross-thread leakage).
- `CSR-CONV-002`: Sidebar previews must load latest message lazily with stable ordering (most recent first).
- `CSR-CONV-003`: Missing preview state must show explicit loading placeholder, not "No messages" until fetch completes.
- `CSR-CONV-004`: Tool/thinking/internal traces must never appear as final assistant chat content.
- `CSR-CONV-005`: LLM role prefixes (`User:`, `Agent:`) must be normalized out before rendering user-visible output.
- `CSR-CONV-006`: Failed thread loads must surface retryable errors with non-destructive fallback.

## C. Prompt queue + steer behavior
- `CSR-QUEUE-001`: Queued prompts must not render/send until their turn or explicit steer.
- `CSR-QUEUE-002`: Editing queued prompts must preserve queue order semantics.
- `CSR-QUEUE-003`: If the first item is under edit when injection turn arrives, skip injection of that item and inject next eligible item.
- `CSR-QUEUE-004`: Queue state and transitions must survive reconnect/reload.
- `CSR-QUEUE-005`: Queue operations must be receipted (`enqueue`, `dequeue`, `edit`, `skip`, `inject`).

## D. Prompt suggestions quality gates
- `CSR-SUG-001`: Suggestions must use only current-conversation context window.
- `CSR-SUG-002`: If conversation has no history, suggestions must be omitted.
- `CSR-SUG-003`: Suggestions must not parrot recent user messages.
- `CSR-SUG-004`: Suggestions must be non-redundant and useful; output fewer than 3 when quality bar is unmet.
- `CSR-SUG-005`: Suggestions must be plain prompt text with no wrapping quotes or role wrappers.
- `CSR-SUG-006`: Suggestion generation must be bounded to last 5-8 turns (configurable).

## E. LLM provider/model management
- `CSR-LLM-001`: Model metadata must include context window, params, locality (local/cloud), specialty tags.
- `CSR-LLM-002`: Cost/power scales must normalize across available catalog (1=lowest, 5=highest).
- `CSR-LLM-003`: Download progress must include percentage and spinner state.
- `CSR-LLM-004`: Download completion must emit user-visible receipt/notice.
- `CSR-LLM-005`: If params unknown, system must run discovery path and persist resolved metadata.
- `CSR-LLM-006`: Subagent model routing must select by task scope, context need, and cost budget policy.
- `CSR-LLM-007`: Startup policy may recommend at least one local model based on host capability.

## F. Channels/extensions/eyes
- `CSR-CH-001`: Channel adapters must register through core registry; UI reflects runtime active set.
- `CSR-CH-002`: Extensions tab state must be backed by runtime source of truth.
- `CSR-EYES-001`: Eyes tab must show active eyes from runtime in real time.
- `CSR-EYES-002`: Eyes manual addition via URL/API key must validate and persist via core authority.
- `CSR-EYES-003`: Eyes failures must not crash dashboard; degraded state must be visible and recoverable.

## G. Gateway/dashboard runtime reliability
- `CSR-RT-001`: Gateway should persist across transient dashboard failures.
- `CSR-RT-002`: Dashboard boot should be single-attempt reliable under normal conditions.
- `CSR-RT-003`: Reboot/restart path must preserve agents, sessions, and queues.
- `CSR-RT-004`: Hard stop semantics must be explicit (`gateway stop`) and receipted.
- `CSR-RT-005`: Runtime disconnect must auto-retry with backoff and state resync.

## H. Security/authority boundaries
- `CSR-SEC-001`: All mutating client commands must route to core lanes.
- `CSR-SEC-002`: Fail-closed semantics must be preserved on lane errors/timeouts.
- `CSR-SEC-003`: Policy and guard evaluation must execute in Rust kernels, not client wrappers.
- `CSR-SEC-004`: Every critical action must emit deterministic receipt hash.

## I. File/layout migration governance
- `CSR-MIG-001`: Migrations are executed largest-file-first queue.
- `CSR-MIG-002`: Each migration unit must define parity checklist before move.
- `CSR-MIG-003`: After each unit: rebuild, targeted tests, churn guard, receipt capture.
- `CSR-MIG-004`: No unrelated refactors in same unit (churn isolation).

## Acceptance verification set
- `npm run -s ops:churn:guard`
- `npm run -s ops:file-size:gate`
- `npm run -s ops:rust-core-file-size:gate`
- `npm run -s ops:client-layer:boundary`
- `npm run -s test:ci` (or focused lane tests for touched surfaces)
- `cargo test -p infring-ops-core <touched_kernel_module>`

## Migration notes
- This file is authoritative for migration sequencing and regression prevention until clauses are merged into canonical `SRS.md` lanes.
