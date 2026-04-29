# CODEX ENFORCER

Owner: Jay  
Effective: March 2026  
Scope: All backlog and sprint execution

## Mandatory Pre-Task Protocol
1. Read this file before starting any coding task.
2. Run an explicit enforcer preflight check before implementation.
3. If blocked, state: `BLOCKED — <exact reason>` and stop.

## Definition of Done Reference
- Canonical DoD policy: `docs/workspace/DEFINITION_OF_DONE.md`
- `done` claims must satisfy both this enforcer and the DoD policy.
- If there is any conflict, use the stricter rule.

## Prompt-Start Review Hook
For every incoming user prompt:
1. Re-read this enforcer before implementation.
2. Emit marker: `[codex_enforcer] reviewed`
3. Then continue with preflight + execution.

## Execution Efficiency Guardrail (Mandatory)
Purpose: prevent long proof churn on tiny deltas.

Before implementation, explicitly classify the task and expected delta:
- `docs_only`
- `small_runtime_patch` (roughly <= 20 LOC and narrow scope)
- `broad_runtime_change`

Required execution behavior:
1. Declare expected impact before deep validation (for example: "expected delta is docs-only" or "small runtime patch").
2. Enforce a 10-minute stop-loss: if no meaningful runtime/product delta is produced within 10 minutes, pause and report status + blocker before continuing.
3. Right-size validation to the task class:
   - `docs_only`: run only formatting/lint/doc/srs/dod gates needed for integrity; skip heavyweight runtime/benchmark loops.
   - `small_runtime_patch`: run targeted tests touching changed modules and one relevant safety/regression check.
   - `broad_runtime_change`: run full gates.
4. Do not run benchmark refresh loops by default unless the change is explicitly performance/benchmark/report focused.
5. Keep behavior changes and artifact/report churn in separate commits whenever possible.
6. Escalate immediately if work drifts from expected class (for example docs-only work turning into benchmark/runtime churn).
7. Do not emit "action-item lists" as the primary output when the scoped request can be fully executed in one go.
8. Action-item lists are allowed only when the scope is too large/risky to execute in one pass; in those cases, each list item must include an explicit execution wave plan.

## Standard Implementation Rules (Mandatory)
- Implement all requested items as production code, not receipt scaffolds.
- Authorized modification scope includes `core/`, `surface/`, `client/`, `apps/`, `adapters/`, `tests/`, and `docs/`.
- You may add crates/packages, change schemas, and remove/replace placeholder flows when needed.
- Enforce Rust-kernel authority and thin-shell boundaries on every implementation.
- Terminology transition rule:
  - Canonical presentation term is `Shell`.
  - Repository path remains `client/**` until an explicit migration program is approved.
- Orchestration Surface (Control Plane) is Rust-first by policy:
  - New control-plane authority and coordination logic must land in `surface/orchestration/src/**` (`.rs`).
  - `surface/orchestration/**` must remain at least `95%` Rust by tracked source lines.
  - TypeScript in `surface/orchestration/scripts/**` is adapter-only and must remain minimal delegation code.
  - If control-plane logic requires TypeScript beyond adapter scope, stop with:
    - `BLOCKED — control-plane authority must be Rust; TS allowed only for minimal gateways`
- Any net-new functionality must be paired with a canonical SRS row update in `docs/workspace/SRS.md` before it can be considered complete (`done`), including acceptance criteria and regression-proof references.
- No new authority may be introduced in the shell surface (`client/**`):
  - Shell code is wrapper/UX/integration only.
  - Any new decision logic, policy logic, state mutation authority, or security-critical logic must land in Kernel authority (`core/**` compatibility path).
  - If a task would place new authority in shell/client, stop with:
    - `BLOCKED — new authority in shell/client is prohibited; move implementation to Kernel authority (`core/**` compatibility path)`
- Do not mark any item `done` unless acceptance criteria are proven by:
  - behavior tests,
  - integration tests,
  - runnable CLI evidence.
- If blocked by missing secrets/tools, stop immediately and report the exact blocker.

## Honesty and Completion Rules
- Never mark any backlog item `done` unless it is fully implemented in code and verified.
- Never claim work that was not implemented.
- Never use placeholders, theater, or status-only updates as completion.
- If implementation is partial, mark it as partial/in-progress with exact remaining gaps.

## Required Proof for Every Completed Task
Completion requires all of the following:
1. Visible `git diff` summary for changed files.
2. Successful build output for relevant targets.
3. Successful test output including:
   - At least 1 regression test.
   - At least 1 sovereignty/security check.
4. Runtime/functionality evidence (CLI output, artifact path, or state output).
5. If functionality is new, a same-revision SRS entry/update in `docs/workspace/SRS.md` with explicit regression-insurance acceptance criteria and evidence paths.

## Backlog Discipline
- Do not move queued work to `done` without proof and operator audit readiness.
- Do not close items based on inferred completion.
- Keep failed or blocked items visible with explicit reasons.

## Codex Assimilation Wave Protocol (Mandatory)
- Codex assimilation must run in waves of **4-8 disjoint file shards** per wave.
- Disjoint means one row per unique file path in the wave (no duplicate paths).
- Every wave must run strict preflight before implementation:
  1. `git status --short` is empty.
  2. `npm run -s ops:churn:guard` passes.
  3. selected ledger rows exist and are `queued`.
- Every wave must run targeted tests for touched files before ledger mutation.
- Only after preflight + targeted tests pass may the wave:
  1. update ledger rows (`queued -> done`),
  2. commit,
  3. push.
- If any step fails, stop with:
  - `BLOCKED — codex wave preflight/test gate failed`

## Rust Migration Rules
- Use real public-source metrics only (tracked source files), not weighted/internal metrics.
- Report `.rs` vs `.ts` lines from tracked files.
- Treat the 50% Rust target as repository-wide source composition, not Kernel-only subsets (`core/**` compatibility path).
- Do not inflate Rust percentage with stubs/scaffolding.
- Before adding any **new file** (`git add` path that did not previously exist), run a projected Rust composition check.
- If the projected repository Rust percentage would drop below **70.0%**, the change is blocked.
- Block message format: `BLOCKED — projected Rust % would fall below 70.0 after adding new files`.
- Do not migrate `client/cognition/adaptive/**` into Rust unless the path is under `client/runtime/systems/adaptive/**`.
- Keep user-flex surfaces (`habits`, `reflexes`, `eyes` user-specific paths) non-Rust by default unless explicitly approved.
- Treat these TCB prefixes as Rust-authoritative migration targets: `client/runtime/systems/security/`, `client/runtime/systems/ops/`, `client/runtime/systems/memory/`, `client/runtime/systems/sensory/`, `client/runtime/systems/autonomy/`, `client/runtime/systems/assimilation/`.
- Keep these surface prefixes TypeScript-first unless explicitly overridden: `client/runtime/systems/ui/`, `client/runtime/systems/marketplace/`, `client/runtime/systems/extensions/`.

## Language Allowlist Rules (Mandatory)
- Approved implementation languages are:
  - Rust (`.rs`) for authority/runtime/Kernel logic (`core/**` compatibility path).
  - TypeScript (`.ts`, `.tsx`) for shell wrappers, UX surfaces, and dev/test tooling where Rust is not the execution host.
- JavaScript is prohibited for authored code:
  - Do not add or modify `.js`, `.jsx`, `.mjs`, or `.cjs` implementation files.
  - The only permitted JavaScript changes are deletion/migration of existing legacy JS to Rust/TypeScript.
- If a task requires introducing or editing authored JavaScript, stop with:
  - `BLOCKED — JavaScript is excluded by language policy (migrate to Rust/TypeScript instead)`

## Behavior-Preserving Migration Rules
- Preserve existing behavior unless a breaking change is explicitly requested.
- Add parity checks when migrating logic between TS and Rust.
- Keep fail-closed security behavior active for gated paths.
- Do not change file types or migrate logic across file types (`.ts`, `.js`, `.rs`, etc.) without explicit operator permission in the task instructions.
- If explicit permission is missing for any file-type change or language migration, mark the task `BLOCKED — missing explicit file-type migration permission` and stop.

## Repository Placement Rules (Mandatory)
- Canonical code locations are limited to: `core/`, `surface/`, `client/`, `tests/`, and `adapters/`.
- `apps/` is app-only. It may contain only standalone apps that run on top of the shell/runtime boundary.
- Any path under `apps/` must be deletable without changing core/surface/client/adapters/tests behavior.
- Any path under `apps/` must be deletable without changing core/client/adapters/tests behavior.
- System code must not import from `apps/**`. If system code needs shared logic, move that logic into `core/`, `surface/`, `client/`, `tests/`, or `adapters/` first.
- `apps/` is never a script/tool dump. Shared helpers, wrappers, and runtime bridges are prohibited in `apps/`.
- Top-level `scripts/` is prohibited. Do not create or reintroduce it.
- CI/dev/test tooling scripts must live under `tests/tooling/scripts/`.
- Runtime/operator utilities must live under `client/runtime/systems/**` (or `core/**` when authoritative).
- If initialization/bootstrap installers need a dedicated surface, use `setup/` as the only root-level exception.
- Ownership boundary axiom:
  - Kernel decides what is true and allowed.
  - Orchestration decides what should happen next.
  - Shell decides how it is shown and collected.
- Canonical ownership rulebook: `docs/workspace/orchestration_ownership_policy.md`.
- Canonical Nexus-Conduit-Checkpoint policy: `docs/workspace/nexus_conduit_checkpoint_policy.md`.
- Canonical Layered Nexus Federation Resolution policy: `docs/workspace/layered_nexus_federation_resolution_policy.md`.
- Canonical Cross-Domain Nexus Route Inventory: `docs/workspace/cross_domain_nexus_route_inventory.md`.
- Canonical Conduit/Scrambler Posture policy: `docs/workspace/conduit_scrambler_posture_policy.md`.
- Canonical Gateway Ingress/Egress Interface policy: `docs/workspace/gateway_ingress_egress_policy.md`.
- Canonical Interface Payload Budget policy: `docs/workspace/interface_payload_budget_policy.md`.
- Canonical Shell-Independent Operation policy: `docs/workspace/shell_independent_operation_policy.md`.
- Canonical Shell UI Projection policy: `docs/workspace/shell_ui_projection_policy.md`.
- Canonical Shell UI Message Detail contract: `docs/workspace/shell_ui_message_detail_contract.md`.
- Cross-boundary path rule:
  - Any module/domain boundary crossing must enter and exit through an explicit Nexus checkpoint surface.
  - Cross-boundary traffic must use Conduit with lease/capability, lifecycle, policy, and receipt context.
  - Cross-boundary routes must declare Conduit/Scrambler security posture; sensitive Core/Orchestration authority routes must not silently downgrade below strong Scrambler posture.
  - Direct code-file-to-code-file cross-module paths are migration debt unless explicitly exempted with owner, expiry, and replacement Nexus checkpoint plan.
- Architecture policy governance rule:
  - `ops:policy-refinement:governance` is the aggregate gate for the Shell projection, Shell UI message/detail, Gateway interface, Interface Payload Budget, Shell amputation, Conduit/Scrambler posture, and cross-domain Nexus route inventory guards.
  - `ops:arch:governance` must run `ops:policy-refinement:governance` before broader architecture boundary checks.
- Placement decision rule:
  - system authority/runtime path => Kernel authority (`core/**` compatibility path)
  - control-plane coordination path (non-authoritative) => `surface/orchestration/**`
  - shell runtime wrappers/UX path => `client/runtime/systems/**` (thin runtime/shell surface only)
  - system authority/runtime path => Kernel authority (`core/**` compatibility path) (or `client/runtime/systems/**` only as thin runtime/shell surface)
  - control-plane coordination path (non-canonical decomposition/coordination/sequencing/recovery/packaging) => `surface/orchestration/**`
  - developer/user operational scripts => shell path `client/`
  - test/CI tooling => `tests/`
  - integration bridges for external software => `adapters/`
  - standalone deletable products only => `apps/`
- initialization/bootstrap only => `setup/`

## File Size Governance Rules (Mandatory)
- Treat oversized files as reliability and velocity risk. Split before they become monoliths.
- Line caps apply to **tracked source files**:
  - `client/runtime/systems/ui/**` `*.ts|*.tsx|*.js|*.jsx|*.css|*.html`: hard cap **500** lines.
  - other `*.ts|*.tsx|*.js|*.jsx`: hard cap **1200** lines.
  - `core/**` `*.rs`: hard cap **500** lines.
  - other `*.rs`: hard cap **1000** lines.
- New source files must start at or below **500** lines.
- Legacy files already above cap are allowed only as migration debt:
  - no net line growth is allowed without an exception.
  - any non-trivial touch should reduce size or extract a module in the same batch.
- If a file exceeds **2x** its cap, prioritize decomposition ahead of feature expansion in that file.

### File Size Exceptions (Allowed, But Controlled)
- Exceptions are allowed only when separation would materially harm correctness or operability (for example: generated sources, protocol/ABI tables that must stay contiguous, parser/state-machine sections that must remain atomic).
- Exception requirements (all required):
  1. Explicit operator approval in the task/prompt.
  2. A top-of-file marker: `FILE_SIZE_EXCEPTION: reason=<...>; owner=<...>; expires=<YYYY-MM-DD>`.
  3. A bounded expiry (default max: 14 days) and a follow-up split task.
  4. Regression coverage proving behavior remains stable.
- If these conditions are not met, stop with:
  - `BLOCKED — file size policy violation (missing cap compliance or exception)`

## Git Hygiene Rules (Mandatory)
- Do not leave path migrations as unstaged delete+untracked churn.
- For any directory/file relocation, stage as one atomic move set immediately (`git add -A <old> <new>` or `git mv`).
- Before reporting completion, run `npm run -s ops:churn:guard`; unresolved move-pair churn is a hard fail.
- If churn guard reports likely unstaged moves, stop and resolve staging before any further feature work.

## Sprint Gate
Each sprint/batch must include:
- At least one regression test.
- At least one sovereignty/security validation.
- Rollback/fallback path when applicable.

## Communication Contract
- Be direct, factual, and auditable.
- Surface risks and blockers immediately.
- Do not hide uncertainty.

## Assistant Final-Response Workflow Policy (Mandatory)
- Every assistant-visible reply must be produced through a selected workflow from the workflow library.
- The default selected workflow may be simple or complex, but no assistant reply path is allowed to bypass workflow selection and workflow finalization.
- Post-workflow prose rewrites are prohibited for user-visible assistant text.
- System-authored fallback text is prohibited in visible chat. When the workflow's final LLM stage is unavailable, skipped, or invoke-failed, emit diagnostics through workflow telemetry / attention queues only; user-visible chat content must come from an LLM finalization stage or remain empty with a non-chat error channel.
- Regression invariant: if `response_workflow.final_llm_response.status == synthesized`, the visible assistant text must come from the workflow-authored response and placeholder text such as “I don't have usable tool findings from this turn yet” must not survive in chat or restored history.
