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
| `FC-A09` | Extract tool schema, tool-call normalization, MCP, command, and skill loading semantics | active | Tooling-layer contract pass added for tool schema registry, tool-call normalization, MCP bridge, custom command/skill loading, and the `local_tooling_surface_guard` composite. |
| `FC-A10` | Extract live runtime loop semantics | active | Runtime-layer contract pass added for request transforms, retry streaming, tool dispatch, lifecycle hooks, conversation persistence, and the `local_runtime_execution_loop` composite. |
| `FC-A11` | Extract user-visible output and observability semantics | active | Observability-layer contract pass added for ChatResponse visibility routing, streaming markdown projection, tool output formatting, trace rate limiting, and the `local_runtime_observability_guard` composite. |
| `FC-A12` | Extract CLI, session, command, and user-prompt ingress semantics | active | Ingress-layer contract pass added for CLI prompt/piped input normalization, interactive session state, command prompt generation, user prompt context assembly, and the `local_coding_ingress_guard` composite. |

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
| `tool_schema_registry` | 0 | `forge_tool_macros`, `forge_domain/tools/catalog`, `forge_domain/tools/definition` | lab contract created | Projects tool names, aliases, descriptions, schemas, and sanitization receipts before agent execution. |
| `tool_call_normalization` | 0 | `forge_domain/tools/call`, `forge_domain/transformer/normalize_tool_args`, `forge_domain/transformer/transform_tool_calls`, `forge_domain/tools/result` | lab contract created | Parses XML/native tool calls, assembles streaming parts, repairs arguments, normalizes names, and attaches reflection to tool errors. |
| `mcp_tool_bridge` | 0 | `forge_domain/mcp`, `forge_services/mcp/manager`, `forge_services/mcp/service`, `forge_services/mcp/tool` | lab contract created | Loads scoped MCP configs, merges/caches by config hash, registers sanitized MCP tool names, captures failed servers, and routes MCP calls. |
| `custom_command_skill_loader` | 0 | `forge_services/command`, `forge_domain/command`, `forge_services/tool_services/skill`, `forge_domain/skill` | lab contract created | Loads built-in/global/local commands with precedence and loads repository skills with exact lookup and cache receipts. |
| `local_tooling_surface_guard` | 1 | `forge_tool_macros`, `forge_domain/tools/*`, `forge_domain/mcp`, `forge_services/mcp/*`, `forge_services/command`, `forge_services/tool_services/skill` | lab contract created | Composite guard that prepares tool schemas, normalized call routing, MCP tools, commands, and skills for local coding. |
| `agent_request_transform_pipeline` | 0 | `forge_app/orch`, `forge_domain/transformer/*` | lab contract created | Applies ForgeCode request transforms before each model call: tool sorting, argument normalization, native/non-native tool projection, image handling, and reasoning/provider transforms. |
| `turn_retry_stream_runner` | 0 | `forge_app/orch`, `forge_app/retry`, `forge_domain/chat_response` | lab contract created | Runs one provider chat turn with configured exponential retry, retry attempt events, streaming delta forwarding, and full message reconstruction. |
| `tool_call_execution_dispatch` | 0 | `forge_app/orch`, `forge_app/tool_registry`, `forge_domain/chat_response`, `forge-tool-retry-message.md` | lab contract created | Dispatches tool calls with task parallelism, non-task sequencing, system-tool UI handshakes, lifecycle events, result ordering, and tool-error attempt tracking. |
| `lifecycle_hook_dispatch` | 0 | `forge_app/app`, `forge_app/hooks/mod`, `forge_domain/hook`, `forge_app/orch` | lab contract created | Dispatches start, request, response, toolcall start/end, and end hooks in order, including end-hook continuation behavior. |
| `conversation_state_persistence` | 0 | `forge_app/app`, `forge_app/orch`, `forge_domain/chat_response` | lab contract created | Persists conversation context, prompts, changed-file notices, metrics, request counts, interruption state, yield decisions, and final upserts. |
| `local_runtime_execution_loop` | 1 | `forge_app/orch`, `forge_app/app`, `forge_app/retry`, `forge_app/tool_registry`, `forge_domain/hook`, `forge_domain/chat_response` | lab contract created | Composite runtime loop that wires request transforms, retry streaming, tool dispatch, lifecycle hooks, and conversation persistence around live local coding agent execution. |
| `chat_response_visibility_router` | 0 | `forge_main/ui`, `forge_main/stream_renderer`, `forge_domain/chat_response` | lab contract created | Routes ChatResponse variants to visible markdown, tool status, retry/interrupt display, dimmed reasoning, completion, or telemetry-only lanes. |
| `streaming_markdown_projection` | 0 | `forge_markdown_stream/lib`, `forge_markdown_stream/renderer`, `forge_markdown_stream/repair`, `forge_main/stream_renderer` | lab contract created | Buffers streamed markdown tokens, repairs malformed code fences, renders parse events, syntax-highlights code, and coordinates spinner-safe output. |
| `tool_output_display_format` | 0 | `forge_display/markdown`, `forge_display/code`, `forge_display/diff`, `forge_display/grep`, `forge_app/fmt/fmt_output` | lab contract created | Formats compact user-visible tool output such as diffs, grep/search results, markdown/code blocks, todo diffs, and plan creation titles. |
| `trace_event_rate_limiter` | 0 | `forge_tracker/can_track`, `forge_tracker/rate_limit`, `forge_tracker/log`, `forge_tracker/dispatch`, `forge_tracker/event` | lab contract created | Bounds trace/usage event emission with tracking-enabled rules, fixed-window rate limits, filtered JSON logging, and dropped-event receipts. |
| `local_runtime_observability_guard` | 1 | `forge_main/ui`, `forge_main/stream_renderer`, `forge_markdown_stream`, `forge_display`, `forge_tracker` | lab contract created | Composite guard that separates visible user output, compact display artifacts, and bounded telemetry from execution receipts. |
| `cli_intent_argument_ingress` | 0 | `forge_main/cli`, `forge_main/main`, `forge_main/state` | lab contract created | Normalizes prompt sources, piped input, cwd, sandbox, conversation id, event JSON, and subcommand routing before coding starts. |
| `interactive_input_session_state` | 0 | `forge_main/input`, `forge_main/prompt`, `forge_main/state`, `forge_main/conversation_selector`, `forge_main/porcelain` | lab contract created | Projects prompt loop outcomes, editor buffer state, app-command parsing, prompt metadata, UI cwd/conversation state, and selectable conversation rows. |
| `command_prompt_generation` | 0 | `forge_app/command_generator`, `forge-command-generator-prompt.md`, `forge-commit-message-prompt.md`, `commands/github-pr-description.md` | lab contract created | Generates command-route prompt artifacts with environment/file snapshots, suggest model config, terminal trace projection, JSON schema responses, and command templates without executing side effects. |
| `user_prompt_context_assembly` | 0 | `forge_app/user_prompt`, terminal context, prompt templates | lab contract created | Builds task/feedback user prompt context, command-expanded events, terminal context, droppable piped input, resume todos, attachments, and file-read metrics. |
| `local_coding_ingress_guard` | 1 | `forge_main`, `forge_app/user_prompt`, `forge_app/command_generator`, templates, commands | lab contract created | Composite guard that wires CLI/session ingress, interactive state, command prompt generation, and user prompt context assembly before policy, tooling, runtime, planning, or coding. |

## Runtime behavior harnesses created

| Harness | Owner | Status | Measures |
| --- | --- | --- | --- |
| `coding_safety_layer_lab_behavior_v1` | `orchestration/src/eval_coding_safety_layer.rs` | behavior harness created | Absolute-path reads, line-range hash receipts, guarded writes, snapshot-before-overwrite, exact-match patching, stale-context rejection, and structured validation command receipts. |

Runner:

`cargo run --manifest-path orchestration/Cargo.toml --bin coding_safety_layer_lab_execute`

Neutral master workflow integration:

| Workflow ID | Integration status | Notes |
| --- | --- | --- |
| `local_coding_program_builder` | ingress/policy/context/tooling/runtime/observability/loop-layer dependency declared | The neutral master workflow now references `local_coding_ingress_guard`, `local_policy_permission_guard`, `local_context_loop_guard`, `local_tooling_surface_guard`, `local_runtime_execution_loop`, `local_runtime_observability_guard`, `plan_artifact_create`, `local_code_edit_execution`, `bounded_repair_loop`, and `checkpoint_handoff`; because it composes a level-2 repair loop, its workflow level remains 3. |

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
- `local_coding_program_builder` now has measurable contracts for CLI/session ingress, user prompt context assembly, safe reads/writes, plan artifacts, bounded repair, undo, clarification, validation, checkpoint handoff, ForgeCode-style runtime-loop behavior, and observability projection, but those contracts still need executable runtime-backed evals before we can claim production parity.

Next source pass:
- Inspect ForgeCode initialization services, changed-file notices, terminal context, and title/commit helpers for remaining pre-runtime parity gaps.

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

## Fifth source pass: tool schema, tool call normalization, MCP bridge, commands, and skills

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_tool_macros/src/lib.rs` | Derives tool descriptions from a declared markdown file or doc comments and fails if no description source exists. | Tool definitions should preserve description provenance instead of relying on untracked prompt text. |
| `crates/forge_domain/src/tools/catalog.rs` | Enumerates built-in tool inputs, aliases legacy names, derives JSON schemas, and encodes rich tool-specific input contracts. | The coding workflow needs a schema registry primitive that makes the exact tool surface measurable. |
| `crates/forge_domain/src/tools/definition/name.rs` | Sanitizes tool names to lower snake-like names and translates Claude MCP `mcp__server__tool` names to Forge legacy MCP names. | Tool routing must normalize names before matching, especially for MCP compatibility. |
| `crates/forge_domain/src/tools/call/parser.rs` | Parses XML-style `<forge_tool_call>` blocks, extracts tool names and arguments, and coerces argument strings to JSON-like values. | Non-native tool-call mode needs explicit normalization receipts. |
| `crates/forge_domain/src/tools/call/tool_call.rs` | Assembles streaming tool-call parts, preserves call IDs and thought signatures, and includes a GLM fragment workaround. | Tool-call normalization must be model-aware and preserve hidden metadata internally. |
| `crates/forge_domain/src/tools/call/args.rs` | Normalizes parsed and unparsed arguments, repairs malformed JSON where possible, and falls back to raw-content objects. | Failed or malformed tool arguments should route through repairable normalization before retry. |
| `crates/forge_domain/src/transformer/normalize_tool_args.rs` | Converts persisted unparsed tool-call arguments into parsed JSON values across conversation context. | Resumed coding sessions need argument normalization before provider transforms. |
| `crates/forge_domain/src/transformer/transform_tool_calls.rs` | Converts tool-call contexts to non-tool-supported message format and clears the tools list. | Tooling surface must distinguish native and non-native tool modes. |
| `crates/forge_domain/src/tools/result.rs` | Tool failures include cause chains and a reflection prompt, while outputs can be text, AI conversation refs, images, or empty values. | Tool error reflection should be attached at the tool-result layer, not only the repair-loop layer. |
| `crates/forge_domain/src/mcp.rs` | Supports local/user MCP scopes, stdio/http configs, disable flags, OAuth modes, and deterministic config hashing. | MCP availability should be a measurable tool-surface dependency. |
| `crates/forge_services/src/mcp/manager.rs` | Reads user/local MCP configs, merges them with local precedence, writes configs, and clears caches. | MCP config handling belongs in a dedicated MCP bridge primitive. |
| `crates/forge_services/src/mcp/service.rs` | Lazily initializes MCP servers, caches grouped tools by config hash, records failed servers, sanitizes generated tool names, and supports legacy lookup. | MCP tool failures and cache state need receipts before coding agents rely on external tools. |
| `crates/forge_services/src/command.rs` | Loads embedded commands, global commands, local cwd commands, parses YAML frontmatter, caches commands, and lets local override global/built-in. | Command loading should be separate from execution and record precedence/parse failures. |
| `crates/forge_services/src/tool_services/skill.rs` | Caches repository-loaded skills, returns exact skill matches, and errors clearly when a skill is missing. | Skill availability should be an inventory/lookup primitive, not an implicit prompt assumption. |

Tooling-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Tool schema and description provenance before execution | `tool_schema_registry` | P0 |
| Tool-name sanitization and legacy alias capture | `tool_schema_registry`, `tool_call_normalization` | P0 |
| Native/non-native tool-call normalization with argument repair | `tool_call_normalization` | P0 |
| Streaming tool-call part assembly with hidden thought-signature handling | `tool_call_normalization` | P0 |
| MCP scoped config merge, disabled-server filtering, cache, and failed-server receipts | `mcp_tool_bridge` | P0 |
| Claude MCP double-underscore legacy lookup support | `mcp_tool_bridge` | P1 |
| Built-in/global/local command loading with local precedence | `custom_command_skill_loader` | P1 |
| Exact skill inventory and missing-skill error receipts | `custom_command_skill_loader` | P1 |

## Sixth source pass: live runtime loop behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_app/src/app.rs` | Loads conversation, environment, current files, custom instructions, config-applied agent, provider credentials, model list, tool definitions, system/user prompts, changed-file notices, metrics, tunables, conversation id, hooks, and orchestrator before spawning the response stream. | A real coding workflow needs a runtime preparation layer before the live loop, not only static file-edit primitives. |
| `crates/forge_app/src/orch.rs` | Runs the core request/tool loop with lifecycle start/request/response/tool/end events, request transforms, retry-streamed chat turns, tool execution, context persistence, max request limits, max tool failure limits, and task-complete signaling. | The master coding workflow needs a dedicated runtime execution loop composite that can be measured separately from planning and safety primitives. |
| `crates/forge_app/src/retry.rs` | Uses exponential retry with configured delay, factor, max attempts, jitter, and retry only for retryable domain errors. | Provider turn execution should expose retry configuration and retry attempt receipts. |
| `crates/forge_app/src/tool_registry.rs` | Routes built-in tools, agent tools, and MCP tools; executes Task and agent tools without timeout using parallel joins; checks restricted permissions before Forge tools; applies timeouts to normal Forge/MCP tools; emits MCP output messages. | Tool dispatch must preserve ForgeCode's execution order and timeout/permission boundaries rather than treating tools as opaque calls. |
| `crates/forge_domain/src/hook.rs` | Defines start, request, response, toolcall start, toolcall end, and end lifecycle events; hooks can be chained and mutate conversation state sequentially. | Runtime lifecycle behavior should be a primitive that records hook ordering and mutation effects. |
| `crates/forge_domain/src/chat_response.rs` | Defines visible task messages, reasoning, task completion, tool start/end, retry attempts, and interruption reasons for max tool failures and max requests. | Runtime output must distinguish visible assistant content from tool telemetry, retry events, task completion, and interrupts. |

Runtime-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Provider/model request transforms before each chat turn | `agent_request_transform_pipeline` | P0 |
| Retry-streamed provider turn with retry attempt telemetry | `turn_retry_stream_runner` | P0 |
| Task calls parallel, non-task calls sequential, results restored to model order | `tool_call_execution_dispatch` | P0 |
| System tool start/end event handshake before execution | `tool_call_execution_dispatch` | P0 |
| Start, request, response, toolcall, and end hooks with ordered mutation | `lifecycle_hook_dispatch` | P0 |
| End hook can add messages and continue the loop | `lifecycle_hook_dispatch`, `conversation_state_persistence` | P0 |
| Conversation persisted before requests, after tool results, and after runtime completion | `conversation_state_persistence` | P0 |
| Max request and max tool failure interrupts are explicit terminal or yield reasons | `conversation_state_persistence`, `local_runtime_execution_loop` | P0 |
| Runtime composite remains separate from architecture/planning decisions | `local_runtime_execution_loop` | P1 |

## Seventh source pass: user-visible output and observability behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_display/src/markdown.rs` | Renders static markdown with termimad, limits excessive newlines, extracts code blocks before markdown rendering, and restores syntax-highlighted code blocks. | Display formatting should be treated as a compact projection layer, not as raw final-answer content. |
| `crates/forge_display/src/code.rs` | Caches syntax/theme resources, detects terminal light/dark mode once with timeout, falls back to dark, and returns plain text for unknown languages. | Syntax highlighting needs deterministic fallback and should not block coding execution. |
| `crates/forge_display/src/diff.rs` | Formats grouped context diffs, colors insertions/deletions, tracks added/removed line counts, and sizes line-number columns from displayed diff context rather than total file length. | Tool output formatting should include compact diff receipts and avoid giant raw file payloads. |
| `crates/forge_display/src/grep.rs` | Parses `path:line:content` rows, groups matches by path, aligns line numbers, highlights regex matches, and treats non-grep rows as raw file paths. | Search output display belongs in an observability/display primitive separate from repo search execution. |
| `crates/forge_markdown_stream/src/lib.rs` | Buffers streaming tokens until complete lines, repairs each line before parsing, renders streamdown events, and flushes remaining parser state on finish. | Runtime markdown projection should be line-buffered and finish-aware instead of writing arbitrary raw deltas. |
| `crates/forge_markdown_stream/src/renderer.rs` | Renders headings, code, lists, tables, blockquotes, think blocks, horizontal rules, links, images, and inline styles with width/margin handling. | Visible streaming output needs a dedicated markdown projection primitive with explicit supported constructs. |
| `crates/forge_markdown_stream/src/repair.rs` | Splits embedded closing code fences only when already inside a code block. | Markdown repair should be scoped and receipt-backed so it does not corrupt normal text. |
| `crates/forge_stream/src/mpsc_stream.rs` | Spawns a bounded channel stream with capacity one and aborts the producer task when the stream is dropped. | Runtime streams need backpressure and abort semantics in their observability contract. |
| `crates/forge_tracker/src/can_track.rs` | Disables tracking for dev builds and version `0.1.0`, treating other versions as production-capable. | Telemetry must have an explicit tracking-enabled decision before dispatch. |
| `crates/forge_tracker/src/rate_limit.rs` | Uses a fixed sixty-second window and drops events after the per-minute limit. | Observability should bound trace volume without blocking execution. |
| `crates/forge_tracker/src/log.rs` | Filters JSON logs to `forge_` targets, writes to tracker-backed PostHog when tracking is enabled, otherwise writes daily local logs. | Trace destination selection and filtering should be contract-owned by observability, not by coding primitives. |
| `crates/forge_main/src/stream_renderer.rs` | Pauses the spinner while streaming markdown is written, resumes only at line boundaries, supports dimmed reasoning streams, and preserves writer byte-consumption semantics when ANSI styling changes output length. | Visible streaming needs spinner-safe boundaries and separate styling for reasoning versus answer markdown. |
| `crates/forge_main/src/ui.rs` | Ignores empty responses, finishes markdown before tool input/output, notifies tool-start guards even on render failure, tracks tool-end events, conditionally suppresses retry errors, prompts continuation on interrupts, and marks conversations finished on task completion. | ChatResponse visibility routing should be its own primitive with explicit visible/telemetry-only channels and continuation stop points. |
| `crates/forge_app/src/fmt/fmt_output.rs` | Shows diffs for overwrite/patch/multi-patch, plan creation as a debug title, todo output, and intentionally suppresses display for many tool operations. | Display artifacts must not be confused with execution receipts; suppressed output should be explicit. |
| `crates/forge_domain/src/result_stream_ext.rs` | Emits streaming content deltas as partial markdown TaskMessages and reasoning deltas as TaskReasoning while reconstructing full content. | Runtime/observability split should preserve partial visible output while retaining full turn reconstruction internally. |

Observability-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Route ChatResponse variants to visible, status, continuation, or telemetry-only lanes | `chat_response_visibility_router` | P0 |
| Preserve tool-start notify handshake even if UI rendering fails | `chat_response_visibility_router` | P0 |
| Stream answer markdown separately from dimmed reasoning | `chat_response_visibility_router`, `streaming_markdown_projection` | P0 |
| Buffer and repair streamed markdown line-by-line, including scoped embedded fence repair | `streaming_markdown_projection` | P0 |
| Pause/resume spinner at safe output boundaries | `streaming_markdown_projection` | P0 |
| Format write/patch outputs as compact diffs with line-count receipts | `tool_output_display_format` | P0 |
| Group and highlight search output without treating display as search execution | `tool_output_display_format` | P1 |
| Suppress display for operations with no compact display artifact while preserving execution receipts | `tool_output_display_format` | P0 |
| Disable dev-build tracking and bound trace events by fixed windows | `trace_event_rate_limiter` | P0 |
| Keep observability separate from planning, coding, validation, and checkpoint handoff | `local_runtime_observability_guard` | P0 |

## Eighth source pass: CLI, session, command, and user-prompt ingress behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_main/src/cli.rs` | Defines direct prompt, piped input, conversation file/id, directory, sandbox, agent, event JSON, interactive-mode detection, and top-level command routes. | Coding workflows need a measurable ingress layer before planning so prompt source, cwd, sandbox, conversation, and subcommand routes are explicit. |
| `crates/forge_main/src/main.rs` | Reads stdin only when piped, trims empty input away, reads config early, resolves cwd/sandbox/directory, initializes API and UI, and exits cleanly on startup errors. | Local coding should normalize process ingress and stop on invalid cwd/sandbox/config before exposing coding tools. |
| `crates/forge_main/src/input.rs` | Loops over prompt reads, continues on empty/continue, exits on exit, tracks successful prompt text, and parses app commands. | Interactive coding needs prompt-loop receipts rather than treating every terminal read as a task. |
| `crates/forge_main/src/prompt.rs` | Projects cwd, git branch, active agent, model, usage tokens, and cost into prompt state. | Session state should preserve compact prompt metadata without leaking hidden prompt internals. |
| `crates/forge_main/src/state.rs` | Tracks cwd and optional conversation id as UI state. | Conversation continuation and cwd are ingress state, not architecture decisions. |
| `crates/forge_main/src/conversation_selector.rs` | Filters conversations to titled/contextful rows, renders porcelain columns, hides UUIDs, and starts cursor at current conversation. | Resume selection needs a structured session primitive with selectable conversation receipts. |
| `crates/forge_main/src/porcelain.rs` | Converts information sections to tabular machine-readable rows with column transforms, truncation, sorting, and uppercase headers. | Ingress/selection display should be a projection layer, not raw conversation state leakage. |
| `crates/forge_app/src/user_prompt.rs` | Detects resume, classifies task vs feedback, renders command and user prompt templates, injects terminal context, adds droppable piped input and resume todos, parses attachments, and records file attachments as read metrics. | User prompt context assembly must happen before runtime execution and remain separate from file mutation and validation. |
| `crates/forge_app/src/command_generator.rs` | Generates shell commands using environment and directory snapshots, suggest/default session model config, terminal command traces, schema-constrained JSON, and parsed command responses. | Command-route ingress must stop before execution and hand generated commands to permission/safety layers. |
| `templates/forge-command-generator-prompt.md` | Converts natural language to safe shell commands, handles malformed/vague/gibberish input, and returns safe echo warnings for destructive operations. | Shell suggestion behavior should be captured as command prompt generation, not confused with shell execution. |
| `templates/forge-commit-message-prompt.md` | Produces a single raw conventional commit message line from diffs, context, recent commits, and branch name. | Commit prompt generation belongs to ingress/template handling until a later workflow explicitly mutates git. |
| `commands/github-pr-description.md` | Defines a command frontmatter block and expands parameters into a PR-description task prompt. | Custom command execution starts with markdown/template expansion and should not skip prompt-context receipts. |

Ingress-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Normalize direct prompt, piped input, event JSON, and interactive-mode routes before planning | `cli_intent_argument_ingress` | P0 |
| Resolve cwd, sandbox, and conversation id before local coding tools are exposed | `cli_intent_argument_ingress` | P0 |
| Treat interactive prompt outcomes, editor buffer state, app command parsing, and conversation selection as session receipts | `interactive_input_session_state` | P0 |
| Assemble task/feedback user prompt context with current date, terminal context, command expansion, droppable piped input, and resume todos | `user_prompt_context_assembly` | P0 |
| Parse rendered prompt attachments and record file attachment reads as metrics with hashes | `user_prompt_context_assembly` | P0 |
| Generate command-route prompts and shell suggestions without executing generated commands | `command_prompt_generation` | P0 |
| Prefer suggest config over default session config for shell command generation | `command_prompt_generation` | P1 |
| Keep ingress separate from policy permission, architecture planning, local file mutation, validation, and runtime loop execution | `local_coding_ingress_guard` | P0 |
