# Coding Behavioral Assimilation v1

Status: pilot behavioral-assimilation package

Purpose: convert observed coding-agent behavior from reference systems into a cohesive Infring runtime model, primitive contract ledger, reference mapping, and implementation blueprint.

This document is the build target for the native coding reset. It supersedes ad hoc prompt patching as the source of truth for coding workflow behavior.

ForgeCode is included as a first-class benchmark reference, not a side note. Its assimilated behavior should be treated as runtime/controller design evidence: configured tools and limits, structured benchmark validation, retry reflection, no-progress interruption, and pending work gates.

Related artifacts:

- `docs/workspace/coding_runtime_model_v1.md`
- `docs/workspace/coding_runtime_behavioral_model_v2.md`
- `docs/workspace/coding_runtime_behavioral_model_v2_1_implementation_blueprint.md`
- `docs/workspace/coding_runtime_implementation_spec_v1.md`
- `docs/workspace/coding_level1_reference_trace_observations.md`
- `docs/workspace/coding_runtime_reference_observations.md`
- `docs/workspace/coding_runtime_live_trace_observations.md`
- `docs/workspace/coding_level1_reference_trace_observations.md`
- `references/coding-agent-systems/runtime_trace_observations.json`
- `references/coding-agent-systems/runtime_live_trace_observations.json`
- `references/coding-agent-systems/level1_reference_runtime_trace_observations.json`
- `references/coding-agent-systems/runtime_trace_schema.json`
- `references/coding-agent-systems/runtime_live_trace_schema.json`
- `references/coding-agent-systems/runtime_trace_harness/trace_coding_runtime.py`
- `references/coding-agent-systems/runtime_trace_harness/live_trace_coding_runtime.py`
- `references/coding-agent-systems/runtime_trace_harness/level1_reference_runtime_trace.py`

## Behavioral assimilation rule

Assimilate runtime behavior, not surface phrasing.

Imported behavior must become one of:

- runtime state
- primitive contract
- tool contract
- receipt schema
- failure reason
- adapter/profile rule
- composite wiring rule
- eval-only fixture

Imported behavior must not become:

- benchmark-specific production logic
- prompt-only success criteria
- language-specific global behavior without an adapter
- workflow prose that bypasses runtime gates

## State machine

### State table

| State | Entry condition | Allowed actions | Exit condition | Terminal? |
|---|---|---|---|---|
| `task_contract` | User task enters coding category. | Parse task kind, write scope, public surface, validation needs. | Contract produced or missing user input. | No |
| `context_selection` | Existing-project work requires local context. | `file_list`, `file_stat`, `file_read`, `file_read_many`, context-provider lookup. | Context pack produced, create-file fast path selected, or blocker. | No |
| `implementation_entry` | Task requires mutation and no mutation receipt exists. | `file_write`, `file_patch`, scoped edit tool. | First successful mutation receipt or blocker. | No |
| `mutation_execution` | A mutation plan or repair action exists. | `file_write`, `file_patch`, exact-context edit, generated-file write. | Source/test/doc/checkpoint receipt produced or tool error. | No |
| `public_interface_verification` | Mutation receipts exist and task/test imports imply public surface. | Language/profile verifier, read changed source/tests if needed. | Surface verified or structured gap. | No |
| `validation_execution` | Validation/test status required after mutation. | `command_run` validation command. | Validation receipt produced. | No |
| `failure_diagnosis` | Tool, interface, or validation failure exists. | Classify failure, select repair reason. | Repair reason emitted or blocker. | No |
| `bounded_repair` | Repair reason exists and budget remains. | Earliest-stage allowed repair action only. | Gap closed, new failure, budget exhausted. | No |
| `checkpoint_handoff` | Long-running/project task requests checkpoint artifacts. | Write checkpoint/handoff artifacts. | Handoff receipt produced. | No |
| `final_receipt_synthesis` | Required receipts/gates are satisfied or terminal blocker exists. | No tools. Synthesize from receipts. | Final user response. | Yes |
| `partial_blocked` | Local execution cannot continue safely. | No tools unless user supplies missing input. | User-visible blocker. | Yes |
| `failed_budget` | Step/repair/time budget exhausted. | No tools. | User-visible partial with receipt refs. | Yes |
| `failed_tooling` | Required primitive/tool unavailable or denied. | No tools. | User-visible tooling failure. | Yes |

### Hard transition rules

- `task_contract -> final_receipt_synthesis` is allowed only for non-mutation answers.
- `context_selection -> validation_execution` is forbidden when `requires_mutation=true` and no mutation receipt exists.
- `implementation_entry -> final_receipt_synthesis` is forbidden until mutation receipt exists or blocker is emitted.
- `validation_execution -> final_receipt_synthesis` is forbidden if validation happened before mutation.
- `validation_execution -> final_receipt_synthesis` is forbidden when public-interface verification has unresolved gaps.
- `failure_diagnosis -> bounded_repair` must target the earliest unsatisfied state.
- `bounded_repair -> context_selection` is allowed only when repair proves missing context.
- `checkpoint_handoff` and memory closure are after source/test/validation gates, never before.

### Controller-owned invariants

- No mutation receipt means no implementation success.
- Passing baseline tests before mutation is not completion evidence.
- Final answers are synthesized from receipts and blockers, not from provider confidence.
- Model output may propose actions, but runtime state decides whether actions are allowed.
- Repeated no-progress transitions become `failed_budget` or `partial_blocked`, not infinite retries.

## Primitive contract ledger

### `coding_task_contract`

Layer: Orchestration/Core Layer 2 boundary.

Inputs:

- user prompt
- workflow metadata
- allowed workspace roots
- permissions

Outputs:

- task kind
- write scope
- context requirement
- mutation requirement
- test/validation requirement
- requested public surface
- stop conditions

Receipts:

- `task_contract_receipt_v1`

Failure modes:

- `missing_user_input`
- `unsupported_task_kind`
- `write_scope_unresolved`

Non-goals:

- Does not inspect large source files.
- Does not decide architecture for multi-slice projects.

Reference sources:

- Continue context/provider surfaces.
- OpenHands event/state task framing.
- ForgeCode task/workflow contract patterns.

### `context_pack_builder`

Layer: Orchestration primitive with tool-backed context providers.

Inputs:

- task contract
- repo root
- candidate paths
- search/provider policy

Outputs:

- selected files
- rationale
- excluded paths
- likely validation commands
- public API owners
- confidence/open questions

Receipts:

- `context_pack_receipt_v1`

Failure modes:

- `context_not_found`
- `ambiguous_context`
- `permission_denied`

Non-goals:

- Does not mutate files.
- Does not run validation as completion evidence.

Reference sources:

- Aider repo map.
- SWE-agent search tools.
- Continue context providers.

### `implementation_entry_gate`

Layer: Core Layer 2 runtime gate.

Inputs:

- task contract
- context state
- receipts
- model output

Outputs:

- allowed action set
- repair reason when mutation is missing
- blocker when mutation is unsafe

Receipts:

- `implementation_entry_gate_receipt_v1`

Failure modes:

- `missing_product_mutation_receipt`
- `mutation_blocked_by_scope`
- `mutation_blocked_by_missing_context`

Non-goals:

- Does not choose full project architecture.
- Does not validate code.

Reference sources:

- mini-SWE-agent step loop.
- OpenHands state/action gate.
- SWE-agent terminal submit boundary.

### `file_mutation_executor`

Layer: Core Layer 2 tool execution surface.

Inputs:

- allowed write scope
- edit request
- file operation

Outputs:

- mutation receipt
- path
- status
- old/new hash when available
- error envelope

Receipts:

- `file_mutation_receipt_v1`

Failure modes:

- `path_out_of_scope`
- `patch_context_not_unique`
- `file_missing_for_patch`
- `permission_denied`

Non-goals:

- Does not infer task success.

Reference sources:

- SWE-agent `str_replace_editor`.
- Aider diff discipline.
- Cline patch executor.

### `public_interface_verifier`

Layer: Orchestration primitive with language/profile adapters.

Inputs:

- task contract public surface
- changed source/test paths
- language profile

Outputs:

- verified public surface
- missing symbols/modules/attrs/constructors

Receipts:

- `public_interface_verification_receipt_v1`

Failure modes:

- `missing_public_interface`
- `wrong_module_surface`
- `missing_constructor_shape`
- `missing_return_field`
- `language_adapter_unavailable`

Non-goals:

- Does not replace semantic validation.
- Does not hardcode benchmark symbols.

Reference sources:

- Aider test/lint repair discipline.
- SWE-agent explicit edit/test loop.
- Infring Level 1 semantic-probe failures.

### `validation_runner`

Layer: Core Layer 2 command runtime.

Inputs:

- validation command
- cwd
- environment
- timeout policy

Outputs:

- command receipt
- exit code
- stdout/stderr summary
- success boolean

Receipts:

- `validation_command_receipt_v1`

Failure modes:

- `validation_failed`
- `command_timeout`
- `command_denied`
- `dependency_missing`

Non-goals:

- Does not count baseline pass as implementation success.

Reference sources:

- SWE-ReX command receipt envelope.
- Aider lint/test flow.

### `failure_diagnosis`

Layer: Orchestration primitive.

Inputs:

- tool receipts
- validation receipt
- public-interface receipt
- task contract

Outputs:

- earliest unsatisfied state
- repair reason
- recommended next allowed action

Receipts:

- `failure_diagnosis_receipt_v1`

Failure modes:

- `ambiguous_failure`
- `unsupported_failure_shape`

Non-goals:

- Does not perform mutation.

Reference sources:

- OpenHands stuck detection.
- ForgeCode repair diagnostics.
- Aider validation repair.

### `bounded_repair_loop`

Layer: Core Layer 2/Orchestration bridge.

Inputs:

- failure diagnosis
- repair budget
- receipts
- task contract

Outputs:

- repair action
- updated receipts
- terminal partial if budget exhausted

Receipts:

- `repair_loop_receipt_v1`

Failure modes:

- `repair_budget_exhausted`
- `repeat_no_progress`
- `needs_user_input`

Non-goals:

- Does not broaden scope silently.

Reference sources:

- mini-SWE-agent step/cost limits.
- OpenHands stuck detection.
- Aider repair loop.

### `incremental_receipt_journal`

Layer: Core Layer 2 runtime.

Inputs:

- model turns
- tool calls
- tool receipts
- terminal statuses

Outputs:

- durable trajectory/journal artifact

Receipts:

- `incremental_receipt_journal_v1`

Failure modes:

- `journal_write_failed`
- `journal_partial_write`

Non-goals:

- Does not become source of truth for workspace content.

Reference sources:

- mini-SWE-agent trajectory save.
- OpenHands event/state/replay.
- Cline session lifecycle.

### `final_receipt_synthesis`

Layer: Orchestration finalization primitive.

Inputs:

- task contract
- receipts
- blockers
- validation/interface status

Outputs:

- user-visible final answer
- public reasoning rollup

Receipts:

- `final_receipt_synthesis_v1`

Failure modes:

- `missing_required_receipts`
- `unresolved_blocker`

Non-goals:

- Does not call tools.
- Does not expose hidden chain-of-thought.

Reference sources:

- SWE-agent submit boundary.
- Infring public reasoning trace contract.

## Reference assimilation matrix

| Reference system | Observed behavior | Infring primitive | Build implication |
|---|---|---|---|
| mini-SWE-agent | step loop with cost/step budgets | `bounded_repair_loop`, `step_budgeted_trajectory_runtime` | Runtime must own budgets, not prompt text. |
| mini-SWE-agent | save trajectory after steps | `incremental_receipt_journal` | Persist observations before risky boundaries. |
| SWE-agent | search tools separate from edit tools | `context_pack_builder` | Discovery and mutation must be separate states. |
| SWE-agent | exact-context editor with undo | `file_mutation_executor` | Patches need deterministic apply/fail behavior. |
| SWE-agent | submit as terminal tool | `final_receipt_synthesis` | Finalization is a boundary, not free prose. |
| SWE-ReX | runtime interface owns command execution | `validation_runner` | Command execution must return structured receipts. |
| SWE-ReX | exit code, output, failure reason | `validation_command_receipt_v1` | Validation failure is structured repair input. |
| Aider | repo map ranks context under budget | `context_pack_builder` | Context selection should be explicit and budgeted. |
| Aider | strict diff/edit rules | `file_mutation_executor` | Use exact, applyable edit contracts. |
| Aider | lint/test feedback loop | `failure_diagnosis`, `bounded_repair_loop` | Validation output should drive repair. |
| OpenHands | action/observation/state/event loop | state machine | Runtime state chooses allowed next actions. |
| OpenHands | stuck detection | `bounded_repair_loop` | No-progress loops become terminal partials or strategy changes. |
| OpenHands | runtime/sandbox boundary | `validation_runner`, `file_mutation_executor` | Execution boundary is distinct from controller. |
| Cline | tool lifecycle in session turn | `incremental_receipt_journal` | Tool calls need lifecycle IDs and durable state. |
| Cline | policy/approval gates | tool permission contracts | Tool permission is policy-owned. |
| Cline | patch executor | `file_mutation_executor` | Patch application is a primitive executor. |
| Continue | context provider registry | `context_pack_builder` | Context should be typed provider output. |
| Continue | tool provider abstraction | tool contracts | Tool invocation should be registered and typed. |

## Implementation blueprint

### Phase 1: stop early finalization

Goal:

Make Level 1 unable to pass through finalization without mutation.

Changes:

- Add `coding_task_contract` extraction as a typed runtime artifact.
- Make `implementation_entry_gate` run before public finalization.
- If mutation is missing, issue mutation-only retry.
- If mutation remains missing after budget, return `partial_blocked`, not success-looking final output.

Primary files:

- `core/layer2/agent_surface/src/agent.rs`
- `core/layer2/agent_surface/src/native_evidence.rs`
- `core/layer2/agent_surface/src/native_prompt_policy.rs`

### Phase 2: explicit primitive contracts

Goal:

Move behavior out of the large workflow prompt into primitive contracts.

Changes:

- Add workflow/JSON contract files for each spine primitive.
- Register primitive levels and dependencies in workflow ledger.
- Keep `coding_project_operator` as composition shell.

Primary paths:

- `orchestration/src/control_plane/workflows/lab/primitives/coding_*`
- `orchestration/src/control_plane/workflows/workflow_registry.json`
- workflow ledger docs

### Phase 3: public interface verifier adapter

Goal:

Convert semantic/public API failures into structured repair before finalization.

Changes:

- Add language/profile adapter boundary.
- Start with Python import/export/attribute checks.
- Keep symbols task-derived or test-derived.

Primary files:

- `core/layer2/agent_surface/src/native_evidence.rs`
- future `core/layer2/agent_surface/src/public_interface_verifier.rs`

### Phase 4: validation/repair loop stabilization

Goal:

Make validation failure repair deterministic and bounded.

Changes:

- Convert validation receipts into failure diagnosis artifacts.
- Restrict repair to earliest failed state.
- Add repeated no-progress detection.

Primary files:

- `core/layer2/agent_surface/src/agent.rs`
- `core/layer2/agent_surface/src/native_tools/command_run.rs`

### Phase 5: compare Infring traces to reference model

Goal:

Make every Level 1-3 run produce comparable runtime trace events.

Changes:

- Extend native journal to align with `runtime_trace_schema`.
- Add event kinds: state_entered, allowed_actions, model_turn, tool_call, tool_result, gate_decision, repair_decision, final_synthesis.
- Build a trace comparator against reference observations.

Primary files:

- `core/layer2/agent_surface/src/agent.rs`
- `references/coding-agent-systems/runtime_trace_schema.json`
- eval harness files

### Phase 6: deepen live behavioral traces

Goal:

Move from source-backed behavioral architecture plus provider-free live probes to full shared-task runtime traces.

Changes:

- Run the shared task suite against every reference system that can be configured locally.
- Capture event timelines for create-file, patch-existing-behavior, validation-repair, and terminal-blocker tasks.
- Record missing dependency/setup blockers explicitly.
- Compare reference event order and timing against Infring native journal events.

Primary paths:

- `references/coding-agent-systems/runtime_trace_harness/live_trace_coding_runtime.py`
- `references/coding-agent-systems/runtime_live_trace_observations.json`
- `docs/workspace/coding_runtime_live_trace_observations.md`

Current phase-6 evidence:

- Provider-free live probes now cover 7 systems with 11 probes.
- 8 probes pass.
- mini-SWE-agent successfully runs the shared task loop for create-file, patch-existing, validation, and terminal-blocker traces.
- Remaining blockers are structured setup/dependency/import blockers for deeper SWE-agent, Aider, and OpenHands probes.

## Acceptance criteria for v1 coding reset

Level 1 is healthy only when:

- `5/5` passes twice in a row,
- no run finalizes before mutation,
- every failure emits a structured terminal reason,
- average time to first mutation is below 30 seconds for simple existing-project tasks,
- no Level 1 fix references fixture names, packages, or symbols in production code.

Promotion beyond Level 1 is blocked until these hold.

## Behavioral assimilation template

Future assimilation efforts should follow this pilot structure:

1. Pull reference artifacts.
2. Define trace schema.
3. Build sensor harness.
4. Capture observations.
5. Draft runtime model.
6. Convert observations into state machine.
7. Define primitive contracts.
8. Build reference-to-Infring matrix.
9. Write implementation blueprint.
10. Only then patch production behavior.
