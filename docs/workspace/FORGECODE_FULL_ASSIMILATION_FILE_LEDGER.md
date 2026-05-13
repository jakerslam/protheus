# ForgeCode Full Assimilation File Ledger

Status: active
Source repository: `local/workspace/assimilations/ForgeCode-Assimilation/target-repo`
Source state: local checkout with `.git`, `Cargo.toml`, `Cargo.lock`, `package.json`, `crates/`, `docs/`, `commands/`, `templates/`, and `scripts/`
Inventory date: 2026-05-13

## Assimilation boundary

This ledger tracks the real ForgeCode repository. It must not be confused with the neutral master coding workflow now named `local_coding_program_builder`.

Naming rule:
- `ForgeCode` / `forgecode_*` refers to upstream ForgeCode source, ForgeCode-specific primitives, or ForgeCode assimilation work.
- `local_coding_program_builder` refers to our neutral master coding workflow that may call ForgeCode-derived primitives but is not itself the upstream ForgeCode workflow.

## Repository inventory summary

Measured source files, excluding `.git`, `target`, and `node_modules`: `917`

Top-level source areas:

| Path | Assimilation status | Intended destination |
| --- | --- | --- |
| `.config` | pending source pass | Environment/config conventions |
| `.devcontainer` | pending source pass | Dev environment notes |
| `.forge` | pending source pass | ForgeCode runtime/config semantics |
| `.github` | pending source pass | CI and release validation reference |
| `benchmarks` | pending source pass | Eval/observability benchmarks |
| `commands` | pending source pass | Command workflow behavior |
| `crates` | active inventory | Runtime/tooling/primitive extraction |
| `docs` | pending source pass | Behavioral documentation evidence |
| `plans` | pending source pass | Planning and execution patterns |
| `scripts` | pending source pass | Operational tooling reference |
| `shell-plugin` | pending source pass | Shell integration behavior |
| `templates` | pending source pass | Prompt/template assimilation |

## Crate inventory

| Crate | First-pass role hypothesis | Assimilation target |
| --- | --- | --- |
| `forge_api` | API/client/runtime interface layer | Tool/runtime integration model |
| `forge_app` | Application assembly | Master workflow orchestration evidence |
| `forge_ci` | CI/validation support | `validation_command_runner` primitive |
| `forge_config` | Configuration loading and policy | `repo_context_assessment` and runtime config |
| `forge_display` | Output rendering | Final handoff/result packaging |
| `forge_domain` | Core domain types and workflow concepts | Primitive contracts and state model |
| `forge_embed` | Embedded assets/prompts | Template assimilation |
| `forge_fs` | Filesystem read/write behavior | `local_code_edit_execution` primitive |
| `forge_infra` | Infrastructure glue | Runtime integration reference |
| `forge_json_repair` | Structured output repair | `failure_diagnosis` / artifact repair primitive |
| `forge_main` | CLI entrypoint | Runtime command model |
| `forge_markdown_stream` | Streaming markdown output | Result streaming/visibility model |
| `forge_repo` | Repository context and source control abstraction | `repo_context_assessment` primitive |
| `forge_select` | Selection/ranking behavior | Workflow/tool selection evidence |
| `forge_services` | Service composition | Agent loop orchestration evidence |
| `forge_snaps` | Snapshot testing | Eval fixture/reference behavior |
| `forge_spinner` | Progress UI | Observability/status projection |
| `forge_stream` | Stream/event handling | Tool/event telemetry model |
| `forge_template` | Template expansion | Prompt/template primitive |
| `forge_test_kit` | Test helpers | Eval harness support |
| `forge_tool_macros` | Tool declaration macros | Tool schema extraction |
| `forge_tracker` | Task/progress tracking | Checkpoint and loop-state tracking |
| `forge_walker` | File walking/repo traversal | `repo_context_assessment` primitive |

## Primitive extraction candidates

| Candidate primitive | Source evidence to inspect first | Why it matters |
| --- | --- | --- |
| `repo_context_assessment` | `forge_repo`, `forge_walker`, `forge_config` | Detect stack, files, boundaries, commands, and repo constraints before coding. |
| `architecture_contract_definition` | `forge_domain`, `plans`, `docs` | Turn repo/user goal into explicit architecture and boundary rules. |
| `implementation_slice_planning` | `forge_services`, `forge_domain`, `plans` | Convert a checkpoint into bounded coding slices. |
| `local_code_edit_execution` | `forge_fs`, `forge_services`, `forge_tool_macros` | Core missing primitive for real coding: read/write local files from slice prompts. |
| `validation_command_runner` | `forge_ci`, `.github`, `scripts` | Run repo-native checks and capture structured validation receipts. |
| `failure_diagnosis` | `forge_json_repair`, `forge_services`, `forge_ci` | Diagnose compile/test/tool failures and route repair. |
| `bounded_repair_loop` | `forge_tracker`, `forge_services`, `forge_domain` | Retry without scope creep and stop at explicit barriers. |
| `checkpoint_handoff` | `forge_display`, `forge_stream`, `forge_tracker` | Summarize completed checkpoint, changed files, risks, and next slice. |

## Current assimilation work items

| ID | Work item | Status | Notes |
| --- | --- | --- | --- |
| `FC-A01` | Preserve upstream ForgeCode identity separately from our master coding workflow | complete | Master workflow renamed to `local_coding_program_builder`. |
| `FC-A02` | Build source file ledger and crate map | active | This file is the initial ledger. |
| `FC-A03` | Inspect `forge_fs` and `forge_repo` for real local read/write and repo-context behavior | active | First pass started with `forge_fs`, `forge_services/tool_services`, and `forge_walker`. |
| `FC-A04` | Extract `local_code_edit_execution` primitive contract | active | Initial lab workflow contracts created for the coding safety layer. |
| `FC-A05` | Extract validation/repair loop semantics from `forge_ci`, `forge_services`, and `forge_tracker` | active | Loop-layer contract pass added for plan artifacts, undo, followup clarification, failure diagnosis, bounded repair, and checkpoint handoff. |
| `FC-A06` | Implement measurable coding safety behavior in eval ownership | active | `eval_coding_safety_layer` now exercises safe reads, guarded writes, exact-match patches, stale-context rejection, snapshots, hashes, and validation command receipts. |
| `FC-A07` | Extract prompt, context, tool-routing, and agent-delegation semantics | active | Context-layer contract pass added for tool access resolution, doom-loop interruption, pending todo gating, compaction summaries, tool-error reflection, delegated agent tasks, and the `local_context_loop_guard` composite. |
| `FC-A08` | Extract config and operation-permission semantics | active | Policy-layer contract pass added for ForgeCode config resolution, operation permission gating, and the `local_policy_permission_guard` composite. |

## Assimilated workflow contracts created

These are lab contracts only. They do not yet provide a full ForgeCode runtime clone; they establish the measurable primitive interfaces needed to replace deterministic code materialization with guarded local coding execution.

| Workflow ID | Level | Source evidence | Status | Purpose |
| --- | --- | --- | --- | --- |
| `repo_context_assessment` | 0 | `forge_walker`, `forge_services/fs_search`, `forge_repo/context_engine` | lab contract created | Budgeted, ignore-aware repo context discovery before coding. |
| `safe_file_read` | 0 | `forge_fs/read`, `forge_fs/read_range`, `forge_services/fs_read` | lab contract created | Absolute-path, budgeted reads with range and content-hash receipts. |
| `safe_file_write` | 0 | `forge_fs/write`, `forge_services/fs_write` | lab contract created | Guarded writes with overwrite policy, snapshots, validation errors, and hashes. |
| `safe_file_patch` | 0 | `forge_services/fs_patch` | lab contract created | Exact-match patching with no-match, multiple-match, stale-context, and validation receipts. |
| `validation_command_runner` | 0 | `forge_services/shell`, `forge_ci` | lab contract created | Structured command validation with stdout, stderr, exit code, ANSI handling, and command receipts. |
| `local_code_edit_execution` | 1 | `forge_services/tool_services/*` | lab contract created | Composite local code-edit execution through `safe_file_read`, `safe_file_write`, `safe_file_patch`, and `validation_command_runner`. |
| `safe_file_undo` | 0 | `forge_services/fs_undo`, `forge_services/fs_remove` | lab contract created | Snapshot-backed undo and deletion recovery semantics for repairs that make a checkpoint worse. |
| `plan_artifact_create` | 0 | `forge_services/plan_create` | lab contract created | Dated, non-overwriting plan artifact creation before implementation slices. |
| `followup_clarification_gate` | 0 | `forge_services/followup` | lab contract created | Bounded clarification only when ambiguity blocks safe progress. |
| `failure_diagnosis` | 0 | `forge_services/tool_services`, `forge_ci`, `forge_tracker` | lab contract created | Classifies stale context, ambiguous patch, validation failure, snapshot recovery, user-decision, and unrecoverable blocker cases. |
| `bounded_repair_loop` | 2 | `forge_services/tool_services`, `forge_tracker`, `forge_ci` | lab contract created | Composite diagnose-repair-validate loop with retry budgets, undo/escalation, and no scope expansion. |
| `checkpoint_handoff` | 0 | `forge_tracker`, `forge_display`, `forge_stream` | lab contract created | Packages completed checkpoint, changed files, validation receipts, risks, excluded scope, and next checkpoint. |
| `tool_access_resolver` | 0 | `forge_app/tool_resolver`, `forge_app/system_prompt`, prompt templates | lab contract created | Resolves agent tool access through configured tool patterns, aliases, glob matching, dedupe, ordering, and prompt projection. |
| `doom_loop_interrupt` | 0 | `forge_app/hooks/doom_loop`, `forge-doom-loop-reminder.md` | lab contract created | Detects repeated tool-call signatures and injects an alternate-approach reminder before the next request. |
| `pending_todo_completion_gate` | 0 | `forge_app/hooks/pending_todos`, `forge-pending-todos-reminder.md` | lab contract created | Blocks premature completion when pending or in-progress todos remain. |
| `context_compaction_summary` | 0 | `forge_app/hooks/compaction`, `forge_app/compact`, summary template | lab contract created | Compacts long coding context into summary frames while preserving latest file operations and reasoning continuity. |
| `tool_error_reflection` | 0 | `forge-tool-retry-message.md`, `forge-partial-tool-error-reflection.md` | lab contract created | Requires root-cause reflection and corrected call planning before retrying failed tool calls. |
| `agent_task_delegation` | 0 | `forge_app/agent_executor`, `forge_app/agent` | lab contract created | Executes agent-as-tool tasks with conversation reuse, nested event forwarding, interruption handling, and empty-output rejection. |
| `local_context_loop_guard` | 1 | `forge_app/system_prompt`, `forge_app/tool_resolver`, `forge_app/hooks/*`, `forge_app/agent_executor` | lab contract created | Composite guard that wires tool access, loop interruption, todo gating, compaction, tool-error reflection, and delegation around long coding runs. |
| `forge_config_resolution` | 0 | `forge_config/config`, `forge_config/reader`, `forge_config/writer`, defaults, `forge_services/app_config`, `forge_app/agent` | lab contract created | Resolves layered config, runtime budgets, tool support, restricted mode, model config, reasoning, compaction, and subagent flags. |
| `operation_permission_gate` | 0 | `forge_services/policy`, `permissions.default.yaml`, `forge_domain/policies/*`, `forge_app/services` | lab contract created | Gates read/write/execute/fetch operations through allow, deny, confirm, and accept-and-remember policy behavior. |
| `local_policy_permission_guard` | 1 | `forge_config`, `forge_services/policy`, `forge_domain/policies` | lab contract created | Composite guard that resolves runtime config and operation permissions before local coding execution. |

## Runtime behavior harnesses created

| Harness | Owner | Status | Measures |
| --- | --- | --- | --- |
| `coding_safety_layer_lab_behavior_v1` | `orchestration/src/eval_coding_safety_layer.rs` | behavior harness created | Absolute-path reads, line-range hash receipts, guarded writes, snapshot-before-overwrite, exact-match patching, stale-context rejection, and structured validation command receipts. |

Runner:

`cargo run --manifest-path orchestration/Cargo.toml --bin coding_safety_layer_lab_execute`

Neutral master workflow integration:

| Workflow ID | Integration status | Notes |
| --- | --- | --- |
| `local_coding_program_builder` | policy/context/loop-layer dependency declared | The neutral master workflow now references `local_policy_permission_guard`, `local_context_loop_guard`, `plan_artifact_create`, `local_code_edit_execution`, `bounded_repair_loop`, and `checkpoint_handoff`; because it composes a level-2 repair loop, its workflow level remains 3. |

## Second source pass: planning, repair, undo, and tracker loop behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_services/src/tool_services/fs_undo.rs` | Requires absolute paths, reads before/after state when present, and delegates snapshot restoration through an undo repository. | Repair loops need an explicit undo primitive instead of relying on ad hoc rewrites or blind retries. |
| `crates/forge_services/src/tool_services/fs_remove.rs` | Requires absolute paths, snapshots current content, removes the file, and returns the removed content. | Destructive file operations must be recoverable and receipt-bearing. |
| `crates/forge_services/src/tool_services/followup.rs` | Provides free-text, select-one, and select-many clarification tools, with user-selection receipt text. | Coding workflows should ask only when safe progress is blocked by a real decision. |
| `crates/forge_services/src/tool_services/plan_create.rs` | Writes dated plan artifacts under the plans directory, creates the directory, and refuses to overwrite an existing plan path. | The master coding workflow should persist checkpoint plans before implementation slices. |
| `crates/forge_tracker/src/event.rs` | Normalizes tracked event names and captures start, tool call, prompt, error, trace, and login events. | Checkpoint execution should produce compact structured lifecycle events without leaking raw internal state. |
| `crates/forge_tracker/src/dispatch.rs` | Dispatches tracker events with model/conversation/system metadata and rate-limits event volume. | Loop telemetry should be bounded and explicitly non-authoritative for final answer content. |
| `crates/forge_ci/src/workflows/ci.rs` | Encodes build, test, coverage, prompt benchmark, toolchain, and warning-deny CI conventions. | Validation routing should prefer repo-native checks and preserve command receipts. |
| `crates/forge_ci/src/jobs/lint.rs` | Defines formatting, clippy, and string-safety lint commands with fix/check modes. | Validation command selection should distinguish read-only checks from fix-capable commands and avoid unapproved mutation. |

Loop-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Non-overwriting dated plan artifacts before implementation | `plan_artifact_create` | P0 |
| Snapshot-backed undo after destructive or worsening repairs | `safe_file_undo`, `bounded_repair_loop` | P0 |
| Failure classification before retry | `failure_diagnosis`, `bounded_repair_loop` | P0 |
| Clarification only for blocking ambiguity or user-owned decisions | `followup_clarification_gate` | P0 |
| Retry budgets scoped to the current checkpoint and failing slice | `bounded_repair_loop` | P0 |
| Validation receipts after repair attempts | `bounded_repair_loop`, `validation_command_runner` | P0 |
| Final checkpoint handoff with changed files, validation, risks, excluded scope, and next checkpoint | `checkpoint_handoff` | P0 |

## First source pass: local file and repo tooling

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_fs/src/read.rs` | Provides async raw byte reads, lossy UTF-8 reads, and strict string reads with contextual errors. | Our read primitive needs separate raw, lossy text, and strict text modes rather than a single naive read path. |
| `crates/forge_fs/src/read_range.rs` | Reads 1-based inclusive line ranges, rejects zero indexes, rejects invalid ranges, detects binary files, returns full-file content hash with range metadata. | `repo_context_assessment` and `local_code_edit_execution` should track line-range receipts and content hashes so edits can detect stale context. |
| `crates/forge_fs/src/write.rs` | Provides create-dir, write, append, and remove file operations with contextual errors. | Low-level filesystem primitive is simple, but higher service layer adds safety semantics. |
| `crates/forge_services/src/tool_services/fs_read.rs` | Requires absolute paths, enforces max file/image sizes, detects MIME type, supports image/PDF payloads, resolves default/max line ranges, truncates long lines, returns content hash and line metadata. | Our local read tool should enforce absolute paths, size budgets, binary/image handling, line budgets, and hash-bearing observations. |
| `crates/forge_services/src/tool_services/fs_write.rs` | Requires absolute paths, validates content through a validation repository, creates parent dirs, rejects overwrite unless allowed, snapshots existing files before overwrite, preserves existing line-ending style, returns previous content, validation errors, and content hash. | `local_code_edit_execution` must not be just `std::fs::write`; it needs overwrite policy, snapshot/undo, validation hooks, line-ending preservation, and before/after receipts. |
| `crates/forge_services/src/tool_services/fs_patch.rs` | Uses exact-match patching, normalizes line endings, errors on no match, multiple matches, swap target absence, or out-of-bounds ranges; imports snapshot, fuzzy-search, and validation repositories. | Patch primitive should prefer exact contextual edits, require specificity on multiple matches, and route stale/no-match failures into diagnosis rather than blind rewriting. |
| `crates/forge_services/src/tool_services/fs_search.rs` | Uses regex search with path existence checks, file walking, glob/type filters, output modes, and binary-file skipping. | Search primitive should expose structured modes and integrate with repo walker instead of shelling out blindly. |
| `crates/forge_services/src/tool_services/shell.rs` | Validates non-empty commands, delegates command execution through infra, strips ANSI by default, passes env vars, and returns stdout/stderr/exit code plus shell metadata. | Validation command runner should preserve full command receipts and sanitize output for downstream reasoning. |
| `crates/forge_walker/src/walker.rs` | Walks using ignore filters, excludes `.git`, handles hidden files, symlinks, depth, breadth, file count, file size, total size, and binary extension skipping. | Repo context primitive should be budgeted and ignore-aware, not an unconstrained recursive listing. |

Initial parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Absolute path enforcement for file tools | `local_code_edit_execution` | P0 |
| Read receipts include line range, total lines, and content hash | `repo_context_assessment`, `local_code_edit_execution` | P0 |
| Write receipts include previous content when overwriting and new content hash | `local_code_edit_execution` | P0 |
| Snapshot before destructive overwrite | `local_code_edit_execution`, `bounded_repair_loop` | P0 |
| Exact-match patch operation with multiple-match/no-match errors | `local_code_edit_execution` | P0 |
| Validation hook before/after writes | `validation_command_runner` | P0 |
| Size, line, binary, and hidden-file budgets | `repo_context_assessment` | P1 |
| Structured shell output with stdout/stderr/exit code | `validation_command_runner` | P1 |
| ANSI stripping by default for model-readable validation receipts | `validation_command_runner` | P1 |
| Repo walker that respects ignore rules and hard budgets | `repo_context_assessment` | P1 |

## Assimilation compatibility notes

Known compatibility constraint:
- We should assimilate behavior into measurable primitives, not copy ForgeCode byte-for-byte into the master workflow. Byte-for-byte cloning would make ownership, testing, and promotion boundaries harder to track.

Current blocker for parity:
- `local_coding_program_builder` has measurable contracts for safe reads/writes, plan artifacts, bounded repair, undo, clarification, validation, and checkpoint handoff, but it still does not invoke a live coding agent runtime with real tool calls and iterative repair in production.

Next source pass:
- Inspect ForgeCode prompt/template assets and service composition paths to assimilate how the agent is instructed to choose tools, preserve context, and decide when to stop.

## Third source pass: prompt, context, tool routing, and delegation behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `templates/forge-custom-agent-template.md` | Projects system info, available tools, project guidelines, non-negotiable rules, parallel-tool guidance for supported models, larger reads over tiny reads, no unnecessary file creation, and continuation until objective completion. | Local coding needs an explicit context guard that makes tool access and behavioral constraints measurable before planning/coding starts. |
| `templates/forge-partial-tool-use-example.md` | Defines strict non-native tool-call formatting, required JSON fields, and one-call-per-message behavior when native tool support is unavailable. | Tool access resolution must record prompt projection mode rather than assuming native tool calling. |
| `templates/forge-partial-system-info.md` | Emits OS, cwd, shell, home, selected file list, and git-tracked extension statistics. | Repo context should include compact environment and language-shape signals. |
| `crates/forge_app/src/system_prompt.rs` | Renders static and non-static system blocks, filters tool names to the agent's actual tools, fetches skills, and computes extension statistics with `git ls-files`. | The coding workflow should not expose all tools implicitly; it should resolve agent-specific tool access and record receipts. |
| `crates/forge_app/src/tool_resolver.rs` | Resolves agent tool lists through aliases, glob patterns, dedupe, and agent-defined order. | Tool routing should be a primitive with measurable allowed-tool outputs. |
| `crates/forge_app/src/hooks/doom_loop.rs` | Detects repeated tool-call signatures and repeated recent patterns at threshold 3, then injects a reminder before the next request. | Long coding loops need a distinct doom-loop interrupt primitive separate from repair diagnosis. |
| `crates/forge_app/src/hooks/pending_todos.rs` | Blocks completion when pending or in-progress todos remain, suppressing duplicate reminders unless the todo set changes. | Checkpoint completion must be gated by active task state, not just final answer generation. |
| `crates/forge_app/src/hooks/compaction.rs` | Triggers compaction after responses when the agent compact policy threshold is met. | Context continuity should be handled as a lifecycle hook, not as ad hoc final-answer summary. |
| `crates/forge_app/src/compact.rs` | Compacts message ranges into summary frames, filters droppable messages, transforms redundant file operations, preserves the last reasoning chain, and rolls up usage. | Sophisticated long coding requires compaction that preserves actionable file context and avoids reasoning accumulation. |
| `templates/forge-tool-retry-message.md` and `templates/forge-partial-tool-error-reflection.md` | Failed tool calls include remaining attempts and require explicit reflection on wrong tool, missing parameters, malformed structure, or misread context before retry. | Tool retry should become a reflection primitive instead of blind retry. |
| `crates/forge_app/src/agent_executor.rs` | Agent-as-tool execution can reuse conversations, create agent-initiated conversations, forward nested tool events, return task-completed output, reject empty output, and surface interruptions. | Delegated agent coding needs a bounded primitive with receipts and explicit empty-output/interruption failures. |

Context-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Agent-specific tool catalog resolution before coding | `tool_access_resolver` | P0 |
| Explicit prompt projection mode for native vs non-native tool support | `tool_access_resolver` | P0 |
| Doom-loop detection for identical and patterned tool calls | `doom_loop_interrupt` | P0 |
| Completion blocked by pending/in-progress todos | `pending_todo_completion_gate` | P0 |
| Context compaction preserving latest file operations and reasoning continuity | `context_compaction_summary` | P0 |
| Tool failure reflection before retry | `tool_error_reflection` | P0 |
| Delegated agent tasks with conversation reuse and interruption receipts | `agent_task_delegation` | P1 |

## Fourth source pass: config and permission behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_config/src/config.rs` | Defines runtime budgets, tool support, restricted mode, session/commit/suggest model configs, reasoning, compaction, provider overrides, todo verification, research subagent, and subagent flags. | The coding workflow needs a config-resolution primitive that projects budgets and feature flags before local coding begins. |
| `crates/forge_config/src/reader.rs` | Resolves base path from `FORGE_CONFIG`, legacy `~/forge`, or `~/.forge`; loads `.env` files walking up from cwd; merges legacy, defaults, global TOML, and `FORGE_` env variables. | Config must be layered and receipt-backed, not treated as a static constant. |
| `crates/forge_config/src/writer.rs` | Writes config TOML with a schema header and creates parent directories. | Config mutation should be explicit and auditable, not a side effect of reading config. |
| `crates/forge_config/.forge.toml` | Establishes defaults for read/search/shell/fetch budgets, max requests/tool failures, tool timeout, tool support, todo verification, compaction, retry, HTTP, and reasoning. | Runtime coding limits should be derived from config instead of hardcoded in the master workflow. |
| `crates/forge_services/src/app_config.rs` | Projects session, commit, suggest, and reasoning config through service methods and supports runtime config updates. | Model/reasoning selection belongs in config projection, not prompt text. |
| `crates/forge_app/src/agent.rs` | Applies config to agents, with config filling unset agent fields and explicit reasoning disable overriding agent settings. | Agent execution should receive config-applied budgets and reasoning behavior before task loops. |
| `crates/forge_services/src/policy.rs` | Loads or creates permission policies, evaluates operations, prompts user on confirm, and can accept-and-remember by writing a derived policy. | Authorization needs a separate operation permission primitive before file, shell, or fetch execution. |
| `crates/forge_services/src/permissions.default.yaml` | Defaults allow reads, writes, commands, and fetches broadly. | Default policy behavior must be explicit so restricted mode can be measured and changed safely. |
| `crates/forge_domain/src/policies/engine.rs` | Deny and confirm return immediately; allow is remembered and returned if no deny/confirm matches; no match defaults to confirm. | The permission gate must preserve ForgeCode rule precedence exactly. |
| `crates/forge_domain/src/policies/operation.rs` | Policy operations are read, write, execute, and fetch with cwd/message context. | Permission receipts should classify operation kind and target before execution. |

Policy-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Layered config resolution with defaults/global/env receipts | `forge_config_resolution` | P0 |
| Project tool-supported and restricted-mode flags before local coding | `forge_config_resolution` | P0 |
| Project read/search/shell/fetch/tool-timeout budgets from config | `forge_config_resolution` | P0 |
| Gate read/write/execute/fetch before execution | `operation_permission_gate` | P0 |
| Preserve deny/confirm precedence and allow fallback behavior | `operation_permission_gate` | P0 |
| Support accept, reject, and accept-and-remember user choices | `operation_permission_gate` | P0 |
| Derive remembered policy rules from extension, host, or command prefix | `operation_permission_gate` | P1 |
