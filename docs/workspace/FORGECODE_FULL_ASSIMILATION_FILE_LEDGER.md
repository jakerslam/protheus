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
| `FC-A13` | Extract pre-runtime session bootstrap semantics | active | Session-layer contract pass added for provider/model binding, conversation bootstrap, terminal command snapshots, external file-change notices, title/commit helpers, and the `local_coding_session_bootstrap_guard` composite. |
| `FC-A14` | Extract remote workspace, semantic search, provider transport, and auth service boundaries | active | Remote-service contract pass added for workspace auth, sync/indexing, semantic search, provider transport, and the `local_coding_remote_service_guard` composite. |
| `FC-A15` | Extract sandbox, command surface, update/editor/auth, and data-generation operator integrations | active | Operator-integration contract pass added for git worktree sandboxing, command projection, update/editor/OAuth boundaries, schema data generation, and the `local_coding_operator_integration_guard` composite. |
| `FC-A16` | Extract zsh shell-plugin terminal integration semantics | active | Shell-terminal contract pass added for preexec/precmd terminal context capture, colon-command dispatch, completion/buffer projection, environment diagnostics, and the `local_coding_shell_terminal_guard` composite. |
| `FC-A17` | Extract embedded prompt template and command-template semantics | active | Prompt-template contract pass added for embedded template registration, XML-like prompt element rendering, system/skill prompt projection, recovery templates, summary frames, title prompts, command frontmatter, and the `local_coding_prompt_template_guard` composite. |
| `FC-A18` | Extract project governance, CI/release, and benchmark/eval semantics | active | Governance/evaluation contract pass added for project-local commands/skills, plan validation, dev/test fixtures, generated CI/release boundaries, bounty automation, benchmark eval tasks, and the `local_coding_governance_evaluation_guard` composite. |

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
| `agent_provider_model_binding` | 0 | `forge_app/agent_provider_resolver`, `forge_app/services`, `forge_app/git_app` | lab contract created | Binds requested or active agents to provider/model pairs with default-session fallback and credential-refresh routing before runtime calls. |
| `conversation_session_bootstrap` | 0 | `forge_app/set_conversation_id`, `forge_app/init_conversation_metrics`, `forge_app/apply_tunable_parameters` | lab contract created | Sets conversation id, starts metrics, applies agent tunables and reasoning, and attaches resolved tool definitions to context. |
| `terminal_command_context_snapshot` | 0 | `forge_app/terminal_context` | lab contract created | Reads zsh-exported terminal command traces from unit-separator environment variables, pads missing metadata, and sorts by timestamp. |
| `external_file_change_notice` | 0 | `forge_app/changed_files`, `forge_app/file_tracking` | lab contract created | Rechecks tracked file hashes, reports unreadable/changed files, updates metrics, and injects droppable re-read notices. |
| `title_commit_helper_generation` | 0 | `forge_app/title_generator`, `forge_app/git_app`, title and commit templates | lab contract created | Generates title/commit helper artifacts with schema fallback, git diff selection, commit provider fallback, retryable empty messages, and explicit commit-approval envelopes. |
| `local_coding_session_bootstrap_guard` | 1 | `forge_app` session/bootstrap helpers, terminal context, changed files, title/commit helpers | lab contract created | Composite guard that prepares provider/model, conversation, terminal, external-change, and helper-route state before policy, tooling, runtime, planning, or coding. |
| `workspace_remote_auth_boundary` | 0 | `forge_services/context_engine`, `forge_services/auth`, `forge_services/provider_auth`, `forge_domain/repo`, `forge_domain/node` | lab contract created | Establishes ForgeServices workspace credentials, user id receipts, provider auth contexts, credential refresh boundaries, and secret redaction before remote actions. |
| `workspace_sync_indexing` | 0 | `forge_services/context_engine`, `forge_services/sync`, `forge_services/fd`, `forge_services/fd_git`, `forge_app/workspace_status`, `forge_domain/node` | lab contract created | Syncs remote indexes through canonical paths, exact/ancestor workspace lookup, git/walker discovery, hash comparison, batched delete/upload, progress, and failed-file receipts. |
| `codebase_semantic_search` | 0 | `forge_services/context_engine`, `forge_domain/repo`, `forge_domain/node` | lab contract created | Runs semantic search against indexed workspaces while preserving query/use-case, workspace freshness, ranking metadata, and local-read dependency boundaries. |
| `provider_transport_boundary` | 0 | `forge_services/provider_service`, `forge_services/auth`, `forge_domain/repo` | lab contract created | Renders configured provider URL templates and model-source URLs, delegates chat/model calls, migrates credentials, and records auth-user service receipts without leaking secrets. |
| `local_coding_remote_service_guard` | 1 | `forge_services/context_engine`, `forge_services/sync`, `forge_services/provider_service`, `forge_services/provider_auth`, `forge_domain/repo`, `forge_domain/node` | lab contract created | Composite guard that isolates remote auth, workspace sync/indexing, semantic search, and provider transport from local file mutation and validation. |
| `sandbox_worktree_isolation` | 0 | `forge_main/sandbox` | lab contract created | Creates or reuses sibling git worktree sandboxes with repo-root checks, branch/worktree conflict handling, canonicalized paths, and created/reused receipts. |
| `operator_command_surface_projection` | 0 | `forge_main/model`, `forge_main/info`, `forge_main/tools_display` | lab contract created | Parses slash/colon commands, routes shell bypass without execution, projects workflow/agent commands, Info sections, tool checkboxes, and failed MCP display summaries. |
| `external_update_editor_auth_boundary` | 0 | `forge_main/update`, `forge_main/vscode`, `forge_main/oauth_callback` | lab contract created | Keeps updates, VS Code extension setup, and localhost OAuth callback handling behind explicit operator boundaries and secret-redaction receipts. |
| `schema_data_generation_pipeline` | 0 | `forge_app/data_gen` | lab contract created | Resolves schema/system/user/input files, parses JSONL, binds output tool schema, runs concurrent provider generation, and emits input/output JSON stream receipts. |
| `local_coding_operator_integration_guard` | 1 | `forge_main` operator helpers and `forge_app/data_gen` | lab contract created | Composite guard that isolates sandboxing, command projection, update/editor/auth, and schema data-generation behavior from local file mutation and validation. |
| `zsh_terminal_context_capture` | 0 | `shell-plugin/lib/context.zsh`, `shell-plugin/lib/config.zsh`, `shell-plugin/lib/helpers.zsh` | lab contract created | Captures preexec/precmd command metadata, OSC 133 markers, bounded ring buffers, and unit-separator child-process exports for terminal context. |
| `zsh_command_dispatcher` | 0 | `shell-plugin/lib/dispatcher.zsh`, `shell-plugin/lib/actions/*` | lab contract created | Parses colon commands, resolves aliases and command types, manages active agent/conversation ids, and routes buffer-projection actions without pretending execution happened. |
| `zsh_completion_buffer_projection` | 0 | `shell-plugin/lib/completion.zsh`, `shell-plugin/lib/bindings.zsh`, `shell-plugin/lib/highlight.zsh` | lab contract created | Projects @file completion, colon-command completion, bracketed-paste formatting, keybindings, and syntax highlighting into explicit buffer receipts. |
| `zsh_environment_doctor` | 0 | `shell-plugin/doctor.zsh`, `shell-plugin/keyboard.zsh`, `shell-plugin/forge.setup.zsh` | lab contract created | Diagnoses zsh/plugin/dependency/theme/keyboard/font readiness without mutating shell config. |
| `local_coding_shell_terminal_guard` | 1 | `shell-plugin` modules | lab contract created | Composite guard that isolates shell-terminal integration from local file mutation, validation, and command execution. |
| `embedded_template_registry` | 0 | `forge_embed`, `forge_template` | lab contract created | Registers embedded Handlebars templates by relative path, enforces UTF-8 template paths/content, and renders escaped XML-like prompt elements. |
| `system_skill_prompt_projection` | 0 | `templates/forge-partial-system-info.md`, `templates/forge-partial-skill-instructions.md`, `templates/forge-partial-tool-use-example.md`, `templates/forge-custom-agent-template.md` | lab contract created | Projects environment, file/extension stats, tool-use instructions, skill inventory, custom rules, and custom-agent prompt sections from measured receipts. |
| `recovery_command_template_projection` | 0 | `templates/forge-tool-retry-message.md`, `templates/forge-partial-tool-error-reflection.md`, `templates/forge-partial-summary-frame.md`, `templates/forge-system-prompt-title-generation.md`, `commands/github-pr-description.md` | lab contract created | Projects retry/reflection prompts, summary frames, title-generation instructions, and command frontmatter templates without executing rendered commands. |
| `local_coding_prompt_template_guard` | 1 | `forge_embed`, `forge_template`, `templates`, `commands` | lab contract created | Composite guard that isolates embedded template registration and prompt/command template projection from runtime execution and local file mutation. |
| `forge_project_local_governance` | 0 | `.forge/commands`, `.forge/skills`, `.config/nextest.toml`, `.devcontainer/devcontainer.json`, `docs/tool-guidelines.md` | lab contract created | Projects project-local command/skill discovery rules, plan validation, tool-description limits, nextest preferences, and devcontainer setup without executing governance actions. |
| `forge_ci_release_boundary` | 0 | `.github/workflows`, `.github/release-drafter.yml`, `.github/dependabot.yml`, `.github/labels.json`, `.github/scripts/bounty`, `.forge/skills/write-release-notes` | lab contract created | Captures generated workflow boundaries, CI validation, release target matrix, package distribution, bounty label rules, release-note generation, and secret redaction. |
| `forge_benchmark_eval_harness` | 0 | `benchmarks`, `benchmarks/evals`, `scripts/benchmark.sh`, `scripts/list-all-porcelain.sh` | lab contract created | Projects eval task parsing, execution logging, timeout/early-exit behavior, tool-use validation rules, performance thresholds, and porcelain inventory checks. |
| `local_coding_governance_evaluation_guard` | 1 | `.forge`, `.github`, `.config`, `.devcontainer`, `benchmarks`, `scripts`, `docs` | lab contract created | Composite promotion-support guard that isolates project governance, CI/release, and benchmark/eval evidence from raw coding execution. |

## Runtime behavior harnesses created

| Harness | Owner | Status | Measures |
| --- | --- | --- | --- |
| `coding_safety_layer_lab_behavior_v1` | `orchestration/src/eval_coding_safety_layer.rs` | behavior harness created | Absolute-path reads, line-range hash receipts, guarded writes, snapshot-before-overwrite, exact-match patching, stale-context rejection, and structured validation command receipts. |

Runner:

`cargo run --manifest-path orchestration/Cargo.toml --bin coding_safety_layer_lab_execute`

Neutral master workflow integration:

| Workflow ID | Integration status | Notes |
| --- | --- | --- |
| `local_coding_program_builder` | ingress/session/remote-service/operator-integration/shell-terminal/prompt-template/policy/context/tooling/runtime/observability/loop-layer dependency declared | The neutral master workflow now references `local_coding_ingress_guard`, `local_coding_session_bootstrap_guard`, `local_coding_remote_service_guard`, `local_coding_operator_integration_guard`, nested `local_coding_shell_terminal_guard`, nested `local_coding_prompt_template_guard`, `local_policy_permission_guard`, `local_context_loop_guard`, `local_tooling_surface_guard`, `local_runtime_execution_loop`, `local_runtime_observability_guard`, `plan_artifact_create`, `local_code_edit_execution`, `bounded_repair_loop`, and `checkpoint_handoff`; because it composes a level-2 repair loop, its workflow level remains 3. Governance/evaluation support is tracked separately in `local_coding_governance_evaluation_guard` so promotion evidence does not pollute raw coding execution. |

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
- `local_coding_program_builder` now has measurable contracts for CLI/session ingress, pre-runtime session bootstrap, remote service boundaries, operator integration boundaries, shell-terminal integration, prompt-template projection, user prompt context assembly, safe reads/writes, plan artifacts, bounded repair, undo, clarification, validation, checkpoint handoff, ForgeCode-style runtime-loop behavior, and observability projection. `local_coding_governance_evaluation_guard` now tracks project governance and promotion/eval evidence separately, but executable runtime-backed evals are still required before we can claim production parity.

Next source pass:
- Run a final source-inventory parity review for any missed ForgeCode source families, then shift from structural assimilation to executable eval coverage and promotion-readiness scoring.

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

## Ninth source pass: pre-runtime session bootstrap behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_app/src/agent_provider_resolver.rs` | Resolves provider/model from requested agent when available, otherwise falls back to session defaults, and errors when no default session exists. | Runtime-ready coding must bind provider/model before planning or chat execution can be considered coherent. |
| `crates/forge_app/src/set_conversation_id.rs` | Copies the conversation id into context so downstream runtime calls are conversation-aware. | Conversation id projection should be a bootstrap receipt, not an incidental side effect. |
| `crates/forge_app/src/init_conversation_metrics.rs` | Sets `metrics.started_at` from local current time converted to UTC. | Session bootstrapping should record a start-time receipt before runtime execution. |
| `crates/forge_app/src/apply_tunable_parameters.rs` | Applies optional agent temperature, top-p, top-k, max tokens, reasoning, and resolved tool definitions to context. | Agent tunables and tool definitions need explicit projection before provider calls. |
| `crates/forge_app/src/terminal_context.rs` | Reads `_FORGE_TERM_COMMANDS`, `_FORGE_TERM_EXIT_CODES`, and `_FORGE_TERM_TIMESTAMPS`; splits on ASCII unit separator, pads missing metadata with zero, and sorts by timestamp. | Terminal command context should be a reusable primitive because both user prompt assembly and command generation draw on it. |
| `crates/forge_app/src/file_tracking.rs` | Rechecks tracked file hashes from raw full file content, handles unreadable files as `None`, buffers reads by configured parallelism, and sorts changes by path. | External changes need deterministic, receipt-backed detection before the agent acts on stale file state. |
| `crates/forge_app/src/changed_files.rs` | Updates stored hashes after notification, renders cwd-relative changed paths, and adds a droppable user warning telling the agent to re-read relevant files. | The coding workflow should surface external edits as context, not silently continue with stale assumptions. |
| `crates/forge_app/src/title_generator.rs` | Generates title JSON through a title template, wraps the user prompt, uses temperature 1.0 and a fresh conversation id, and falls back to trimmed plain text when JSON parsing fails. | Title generation belongs in helper/session support and should not contaminate coding receipts. |
| `crates/forge_app/src/git_app.rs` | Generates commit messages from piped, staged, or unstaged diffs; collects recent commits and branch; truncates diffs at char boundaries; prefers commit config then active agent; retries empty messages; and commits with ForgeCode committer metadata when explicitly called. | Git helpers require a separate contract because commit generation is safe-ish but commit execution is externally visible and must remain approval-gated. |
| `crates/forge_app/src/workspace_status.rs` | Compares absolute local and remote file hashes and derives new, modified, deleted, and in-sync paths. | Workspace sync appears to be a separate remote-service layer, not part of the primitive local coding loop yet. |
| `crates/forge_app/src/services.rs` | Defines service boundaries for providers, app config, MCP, conversations, templates, attachments, workspace sync, and related infra. | Remaining assimilation should distinguish local coding primitives from remote workspace/provider/service boundaries. |

Session-layer parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Bind provider/model from requested agent or default session before runtime execution | `agent_provider_model_binding` | P0 |
| Record missing default session, unavailable provider, and credential refresh requirements as blockers | `agent_provider_model_binding` | P0 |
| Set conversation id and start metrics before chat/runtime loops | `conversation_session_bootstrap` | P0 |
| Apply optional agent tunables, reasoning, max tokens, and tool definitions to context | `conversation_session_bootstrap` | P0 |
| Snapshot terminal command context using unit-separator env vars with metadata padding and timestamp sorting | `terminal_command_context_snapshot` | P0 |
| Detect externally changed tracked files by raw content hash before acting on stale context | `external_file_change_notice` | P0 |
| Inject droppable re-read notices and update stored hashes to avoid duplicate warnings | `external_file_change_notice` | P0 |
| Keep title and commit helper generation separate from coding, validation, and git mutation | `title_commit_helper_generation` | P1 |
| Require explicit parent approval before any commit helper mutates git state | `title_commit_helper_generation` | P0 |
| Keep session bootstrap separate from ingress, permission policy, architecture planning, local file mutation, validation, and runtime loop execution | `local_coding_session_bootstrap_guard` | P0 |

## Tenth source pass: remote workspace, semantic search, auth, and provider transport boundaries

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_services/src/context_engine.rs` | Authenticates ForgeServices workspace access, stores workspace API-key credentials with user id params, finds workspaces by exact path then closest ancestor, syncs only existing workspaces, initializes new workspaces explicitly, queries semantic search, lists/deletes workspaces, and computes workspace status. | Remote workspace behavior should be modeled as a guard because it can support coding context but is not local file truth or validation. |
| `crates/forge_services/src/sync.rs` | Canonicalizes paths, emits progress events, hashes files in a first pass, compares remote hashes, batches deletes, uploads files in configured sequential batches, records failed file statuses, and fails if any files failed. | Workspace sync needs progress and failure receipts, plus an explicit boundary that prevents treating indexing as code correctness. |
| `crates/forge_services/src/fd.rs` | Filters sync files by allowed extensions, excludes symlinks and lock/generated names, errors when no source files are found, and falls back from git discovery to walker discovery. | Remote indexing needs its own file discovery contract separate from repo-context scanning and safe file reads. |
| `crates/forge_services/src/fd_git.rs` | Uses `git ls-files`, rejects non-zero git output, rejects empty tracked-file sets, and resolves discovered files against the workspace root. | Git-backed discovery is preferred for remote sync but must have walker fallback and error receipts. |
| `crates/forge_services/src/auth.rs` | Fetches user info and usage from configured services URL with bearer auth and status-checked HTTP GET. | Auth/user-service calls belong to provider/remote transport, not coding primitives. |
| `crates/forge_services/src/provider_auth.rs` | Initializes API key, Google ADC, OAuth code, device, and Codex device flows; pre-fills existing credentials; stores completed credentials; refreshes OAuth-like credentials with a five-minute expiry buffer and tolerates refresh failures. | Provider auth must keep user-visible auth routes and secret redaction explicit before runtime calls rely on credentials. |
| `crates/forge_services/src/provider_service.rs` | Renders provider and model-source URL templates using credential URL params, projects configured template providers into URL providers, delegates chat/model calls, and forwards credential mutation/migration to repositories. | Provider transport should be separate from runtime loop execution and should not leak URL params or raw provider payloads. |
| `crates/forge_domain/src/repo.rs` | Defines provider repositories and workspace index repositories for auth, workspace creation, upload, semantic search, listing, file hash listing, deletion, and credential migration. | The workflow should distinguish repository/service boundaries from model-facing coding behavior. |
| `crates/forge_domain/src/node.rs` | Defines workspace auth, file upload records, codebase wrappers, search params, workspace info, upload stats, codebase query results, semantic nodes, and ranking metadata. | Remote search results need freshness and ranking receipts and should be followed by local safe reads before edits. |

Remote-service parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Workspace auth stores ForgeServices API key plus user id and blocks remote actions when missing | `workspace_remote_auth_boundary` | P0 |
| Provider auth exposes API key, Google ADC, OAuth code/device, and Codex device flows without leaking secrets | `workspace_remote_auth_boundary` | P0 |
| Provider credential refresh uses a five-minute buffer and reports refresh failures without discarding existing credentials | `workspace_remote_auth_boundary` | P0 |
| Workspace sync canonicalizes paths and requires an existing indexed workspace unless explicitly initializing | `workspace_sync_indexing` | P0 |
| Workspace lookup prefers exact path then closest ancestor workspace | `workspace_sync_indexing`, `codebase_semantic_search` | P0 |
| Remote sync discovers source files through git first, walker fallback, allowed extensions, lockfile filters, and symlink exclusion | `workspace_sync_indexing` | P0 |
| Remote sync compares hashes before delete/upload and uses configured sequential upload batches | `workspace_sync_indexing` | P0 |
| Failed file reads are included in sync/status receipts and fail the sync when present | `workspace_sync_indexing` | P0 |
| Semantic search wraps user id, workspace id, query, use-case, limits, and path filters and preserves ranking metadata | `codebase_semantic_search` | P0 |
| Semantic search must not replace exact local safe reads before file edits | `codebase_semantic_search`, `local_coding_remote_service_guard` | P0 |
| Provider URL templates and model-source templates render only with configured credential params and redact secrets | `provider_transport_boundary` | P0 |
| Provider transport readiness must stay separate from runtime-loop provider turn success | `provider_transport_boundary`, `local_runtime_execution_loop` | P0 |
| Keep remote auth/sync/search/transport separate from local file mutation, validation, and checkpoint handoff | `local_coding_remote_service_guard` | P0 |

## Eleventh source pass: operator integration, sandbox, command surface, and data generation behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_main/src/sandbox.rs` | Requires a git repository, resolves the git root, creates or reuses a sibling sandbox worktree, rejects existing non-worktree targets, checks branches, creates missing branches/worktrees, canonicalizes the final path, and emits created/reused titles. | Sandbox setup needs an operator primitive because it can mutate git/worktree state but must not be confused with coding or validation success. |
| `crates/forge_main/src/model.rs` | Parses slash and colon commands, treats bang-prefixed input as shell bypass, passes non-command messages through, registers workflow and agent commands, sanitizes `agent-*` command names, skips reserved conflicts, and sorts command lists. | Command-surface projection should classify and display routes without executing side effects. |
| `crates/forge_main/src/info.rs` | Displays structured sections with title-case keys and placeholder handling for empty values. | Operator-visible status should be a projection layer, not raw internal state leakage. |
| `crates/forge_main/src/tools_display.rs` | Formats enabled/disabled tools with checkbox-like markers, groups system/agent/MCP tools, and truncates failed MCP server error text. | Tool availability display belongs beside command projection and should not be treated as tool execution readiness alone. |
| `crates/forge_main/src/update.rs` | Skips dev/0.1.0 update checks, observes configured update frequency, checks GitHub release metadata, confirms interactive updates unless auto-update is configured, runs the official installer command only on the update path, and exits after successful update/confirmation. | Update behavior is externally visible and needs explicit confirmation/receipt boundaries. |
| `crates/forge_main/src/vscode.rs` | Detects VS Code terminals from environment variables, checks installed extensions through `code --list-extensions`, and installs `ForgeCode.forge-vscode` only when inside VS Code and missing. | Editor integration should be explicit operator setup, not implicit coding execution. |
| `crates/forge_main/src/oauth_callback.rs` | Starts a local callback server only for loopback/localhost redirect URIs with explicit ports, accepts loopback GET requests on the expected path, validates state and code, handles OAuth errors, times out after 300 seconds, and returns no-store/nosniff HTML responses. | OAuth callback handling needs a secret-redacting auth boundary before runtime or provider behavior can depend on it. |
| `crates/forge_app/src/data_gen.rs` | Resolves schema/system/user/input files relative to cwd, reads JSONL inputs, binds an `output` tool to the schema, renders optional user templates per input, runs concurrent provider calls, extracts tool-call arguments, and emits `{input, output}` JSON objects. | Schema data generation is useful operator capability but must be separated from local code mutation, validation, and checkpoint completion. |

Operator-integration parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Require git repository boundaries and reject unsafe sandbox roots | `sandbox_worktree_isolation` | P0 |
| Reuse existing git worktrees but reject existing non-worktree sandbox paths | `sandbox_worktree_isolation` | P0 |
| Create missing branches/worktrees only through explicit sandbox setup receipts | `sandbox_worktree_isolation` | P0 |
| Parse slash/colon commands and route bang-prefixed shell bypass without executing it | `operator_command_surface_projection` | P0 |
| Register workflow and agent commands with reserved-name conflict protection and stable sorting | `operator_command_surface_projection` | P0 |
| Render Info/tool status as compact display projections, including failed MCP server summaries | `operator_command_surface_projection` | P1 |
| Keep update checks frequency-bound and require confirmation unless auto-update is configured | `external_update_editor_auth_boundary` | P0 |
| Install editor extensions only when VS Code context and missing-extension checks pass | `external_update_editor_auth_boundary` | P0 |
| Accept OAuth callbacks only from loopback GET requests with matching path, state, and code | `external_update_editor_auth_boundary` | P0 |
| Redact OAuth codes, tokens, credential params, and external installer payloads | `external_update_editor_auth_boundary` | P0 |
| Resolve data-generation files relative to cwd and parse JSONL one record per line | `schema_data_generation_pipeline` | P0 |
| Bind schema output through an explicit output tool and emit `{input, output}` JSON records | `schema_data_generation_pipeline` | P0 |
| Keep sandboxing, command projection, external setup, auth callbacks, and data generation separate from local file mutation and validation | `local_coding_operator_integration_guard` | P0 |

## Twelfth source pass: zsh shell-plugin terminal integration behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `shell-plugin/forge.plugin.zsh` | Sources config, highlighting, helpers, terminal context capture, completion, action handlers, dispatcher, and bindings as one modular plugin. | Shell integration should be a composite guard rather than a single ingress rule. |
| `shell-plugin/forge.setup.zsh` | Adds zsh autosuggestions and syntax-highlighting plugins when missing, loads Forge plugin and theme only when not already loaded, and marks the block as managed. | Setup behavior should remain outside normal coding execution and be approval/update owned. |
| `shell-plugin/lib/config.zsh` | Defines hidden plugin variables, session model/provider/reasoning overrides, terminal context switches, ring-buffer arrays, max command count, delimiter, preview window, and bat/cat fallback. | Shell state needs explicit scoped receipts so session overrides and terminal metadata do not leak globally. |
| `shell-plugin/lib/context.zsh` | Uses preexec/precmd hooks, OSC 133 markers, conservative terminal support detection, command/exit/timestamp ring buffers, and max-buffer trimming. | Terminal context capture should be a primitive feeding session bootstrap/user prompt context without replacing safe repo reads. |
| `shell-plugin/lib/helpers.zsh` | Lazily loads command lists, wraps fzf options, executes Forge child processes with active agent and scoped terminal env vars, redirects interactive invocations through `/dev/tty`, and starts background sync/update checks. | Forge child-process invocation is a shell-terminal boundary with explicit environment export and background job receipts. |
| `shell-plugin/lib/dispatcher.zsh` | Parses `:command` buffers, handles `: prompt` default route, stores original history, maps aliases, dispatches built-ins/custom commands/agent selection, generates conversation ids, and resets ZLE buffers. | Colon-command dispatch should be measurable and distinct from executing generated shell commands or coding work. |
| `shell-plugin/lib/completion.zsh` | Handles `@` file completion through `forge list files --porcelain` and fzf previews, wraps selections as `@[path]`, completes colon commands from a lazily cached command list, and falls back to default completion otherwise. | Completion is buffer projection; selected files still need downstream safe read receipts before editing. |
| `shell-plugin/lib/bindings.zsh` | Registers ZLE widgets, maps Enter to Forge accept-line, Tab to Forge completion, and formats bracketed paste through `forge zsh format` only for Forge colon buffers. | Keybindings and paste formatting need UI/buffer receipts and must not mangle ordinary shell commands. |
| `shell-plugin/lib/highlight.zsh` | Adds syntax highlighting patterns for `@[...]`, colon command names, and command arguments. | Highlighting is display projection, not execution state. |
| `shell-plugin/lib/actions/core.zsh` | Handles new/info/dump/compact/retry/help, requires active conversation for conversation commands, and starts background sync/update after interactive prompts. | Conversation-aware shell commands need stop conditions for missing conversation and background-job boundaries. |
| `shell-plugin/lib/actions/git.zsh` | Generates commit messages, and commit-preview writes a `git commit` command into the shell buffer based on staged-state strategy instead of executing it immediately. | Git helper projection belongs behind operator boundaries and should not be treated as unapproved git mutation. |
| `shell-plugin/lib/actions/editor.zsh` | Opens an editor from `FORGE_EDITOR`, `EDITOR`, then `nano`, writes `.forge/FORGE_EDITMSG.md`, cleans up on exit, and projects edited content back into `: prompt` buffer. | External editor composition is an operator action with buffer projection and cleanup receipts. |
| `shell-plugin/lib/actions/auth.zsh` | Selects providers through fzf and routes login/logout through interactive or non-interactive Forge provider commands. | Auth actions must remain behind external auth boundaries and secret-redaction receipts. |
| `shell-plugin/doctor.zsh` | Checks zsh, terminal, Forge binary, plugin load, load order, theme, fzf/fd/bat dependency versions, zsh plugins, editor config, PATH, keyboard meta settings, and font support; exits nonzero only for failures. | Environment diagnostics are a primitive guard, not proof that coding execution works. |
| `shell-plugin/keyboard.zsh` | Renders platform and keymap-specific ZLE shortcuts, detecting macOS/Linux/Windows and vi/emacs mode. | Keyboard guidance belongs to shell diagnostics and operator display. |

Shell-terminal parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Capture terminal command text, timestamps, and exit codes through zsh preexec/precmd hooks | `zsh_terminal_context_capture` | P0 |
| Emit OSC 133 markers only when explicitly enabled or conservatively detected as supported | `zsh_terminal_context_capture` | P0 |
| Export terminal context to child Forge invocations using ASCII unit separator env vars only for that process | `zsh_terminal_context_capture` | P0 |
| Parse colon commands, support `: prompt`, aliases, active-agent selection, custom commands, and conversation id generation | `zsh_command_dispatcher` | P0 |
| Keep commit-preview, suggest, and editor composition as buffer projection routes before execution | `zsh_command_dispatcher`, `zsh_completion_buffer_projection` | P0 |
| Complete `@` file references and colon commands through fzf-backed projection without treating selection as safe file read | `zsh_completion_buffer_projection` | P0 |
| Format bracketed paste only for Forge colon buffers and leave normal shell commands alone | `zsh_completion_buffer_projection` | P0 |
| Diagnose zsh/plugin/dependency/theme/keyboard/font readiness with pass/warn/fail receipts | `zsh_environment_doctor` | P0 |
| Keep shell-terminal integration separate from coding, validation, git mutation, and auth side effects | `local_coding_shell_terminal_guard` | P0 |

## Thirteenth source pass: embedded prompt templates and command templates

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `crates/forge_embed/src/lib.rs` | Recursively walks embedded directories and registers every embedded file into Handlebars using the relative file path as the template name, failing on non-UTF-8 paths/content or template parse errors. | Template availability should be registry-backed and failure-explicit before runtime prompt assembly. |
| `crates/forge_template/src/element.rs` | Builds XML-like elements, maps dot-suffixed names to CSS class attributes, escapes text by default, supports explicit CDATA, appends child elements in order, and renders attributes on separate lines. | Prompt element rendering needs explicit escaping and CDATA routes, not string concatenation. |
| `templates/forge-system-prompt-title-generation.md` | Instructs title generation to produce short title-case technical titles without Markdown or marketing language. | Title helper behavior should be a template primitive and not mixed into coding receipts. |
| `templates/forge-partial-summary-frame.md` | Renders prior messages and tool calls into authoritative summary frames, including reads, writes, deletes, searches, skills, semantic search, shell commands, MCP calls, and todo changes. | Compaction should preserve operational context as structured summary frames while remaining separate from validation. |
| `templates/forge-partial-tool-error-reflection.md` | Requires explicit reflection on tool-call errors, including wrong tool, bad/missing parameters, malformed structure, cause, and corrected call. | Tool retry behavior should force diagnosis before retry, not blind repetition. |
| `templates/forge-tool-retry-message.md` | Reports failed tool calls with attempts remaining and asks the agent to analyze root cause before retrying. | Retry prompts should carry attempt budgets and failure visibility. |
| `commands/github-pr-description.md` | Uses frontmatter for command name/description and expands parameters into a PR-description task prompt. | Command templates need frontmatter parsing and parameter expansion receipts before any external action. |
| `templates/forge-partial-system-info.md` | Projects OS, cwd, shell, home, optional file list, and optional workspace extension stats into system information. | System prompt context must come from measured environment/file receipts. |
| `templates/forge-partial-skill-instructions.md` | Lists available skills and instructs invocation only for skills present in the supplied inventory. | Skill prompt projection must not invent unavailable skills. |
| `templates/forge-partial-tool-use-example.md` | Defines XML-like single-tool-call examples for non-native tool mode. | Tool-use examples should depend on provider/tool support mode. |
| `templates/forge-custom-agent-template.md` | Combines system information, conditional tool support, project guidelines, non-negotiable rules, citation format, tagged-file behavior, and skill/tool guidance. | Custom agent prompt templates should be rendered as artifacts from measured context, not hardcoded in runtime. |

Prompt-template parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Register embedded templates by relative path and fail on non-UTF-8 paths/content | `embedded_template_registry` | P0 |
| Render prompt elements with HTML escaping by default and explicit CDATA only when requested | `embedded_template_registry` | P0 |
| Project system info only from measured environment, file, and extension receipts | `system_skill_prompt_projection` | P0 |
| List only supplied available skills and avoid invoking absent skills | `system_skill_prompt_projection` | P0 |
| Switch tool-use instructions based on native tool support | `system_skill_prompt_projection` | P0 |
| Preserve custom-agent non-negotiable prompt sections without overriding parent policy | `system_skill_prompt_projection` | P0 |
| Carry retry attempt budgets and require root-cause reflection before tool retry | `recovery_command_template_projection` | P0 |
| Preserve summary-frame records for file, search, skill, semantic-search, shell, MCP, and todo tool calls | `recovery_command_template_projection` | P0 |
| Parse command frontmatter and expand parameters without executing rendered command actions | `recovery_command_template_projection` | P0 |
| Keep prompt-template projection separate from runtime execution, validation, permission, and file mutation | `local_coding_prompt_template_guard` | P0 |

## Fourteenth source pass: project governance, CI/release, and benchmark/eval behavior

Evidence files inspected:

| Source file | Observed behavior | Assimilation implication |
| --- | --- | --- |
| `.forge/commands/check.md` | Defines a project-local command with frontmatter and explicit `<lint>` and `<test>` tags for cargo fmt/clippy and insta tests, then instructs fixing found issues. | Project-local commands should expose lint/test command tags as governance metadata, not execute automatically inside raw coding. |
| `.forge/commands/fixme.md` | Defines a simple command to find and fix FIXME comments. | Local commands can be instruction-only and still need frontmatter receipts. |
| `.forge/skills/create-plan/SKILL.md` | Creates planning-only Markdown files under `plans/{date}-{task}-vN.md`, requires checkbox tasks, forbids code snippets and implementation changes, requires source citations, and mandates validation scripts. | Planning governance should remain separate from coding execution and require plan validation receipts. |
| `.forge/skills/create-agent/SKILL.md` | Requires custom agents in `<cwd>/.forge/agents/{agent-id}.md` with YAML frontmatter fields and markdown body structure. | Agent authoring/discovery rules should be captured as project-local governance fixtures. |
| `.forge/skills/create-command/SKILL.md` | Requires commands in `<cwd>/.forge/commands/{command-name}.md`, YAML frontmatter, command body, and optional `<lint>`, `<test>`, and `<shell>` tags. | Command authoring rules should be measurable without executing command bodies. |
| `.forge/skills/write-release-notes/SKILL.md` | Fetches release/PR data, categorizes changes, writes user-facing release notes, filters contributors, validates notes under 2000 chars, and avoids implementation jargon/PR references. | Release notes belong to CI/release governance, with length and style validation receipts. |
| `.config/nextest.toml` | Sets slow-test thresholds and failed-only status/final-status output. | Test runner preferences should be captured as validation-environment fixtures. |
| `.devcontainer/devcontainer.json` | Defines Rust devcontainer image, Node, GitHub CLI, git, zsh/Oh My Zsh, zsh shell defaults, fzf/fd, forgecode, rustfmt/clippy, cargo-insta, cargo-nextest, and ast-grep setup. | Reproducible development environment setup should be a fixture, not assumed by coding primitives. |
| `docs/tool-guidelines.md` | Defines tool-description best practices, including detailed descriptions, selection criteria, parameter clarity, registry registration, and a 1024-character limit. | Tool schema quality should be governed and evaled, not only syntactically registered. |
| `.github/workflows/ci.yml` | Generated workflow warns against hand edits, runs CI on PR/main/tag events, sets `RUSTFLAGS=-Dwarnings`, generates coverage, runs zsh rprompt performance benchmark, drafts releases on main, and conditionally builds PR release artifacts by label. | CI/release workflow files should be treated as generated artifacts and boundary evidence. |
| `.github/workflows/release.yml` | Builds multi-platform release binaries on release publication, uploads binaries, updates NPM package repos, and updates Homebrew formula through secrets. | Release publication and package distribution must stay behind explicit release boundaries and secret redaction. |
| `.github/scripts/bounty/src/rules.ts` | Computes pure label/comment patches for bounty issue/PR sync, including value labels, claimed/rewarded lifecycle, linked issue extraction, and no side effects in rule computation. | GitHub automation should separate pure desired-state rules from mutation application. |
| `benchmarks/task-executor.ts` | Spawns task commands, streams stdout/stderr to stripped-ANSI logs, supports timeout kill, early exit when validations pass, and records duration/error metadata. | Eval execution needs structured receipts and cannot be reduced to pass/fail only. |
| `benchmarks/parse.ts` | Parses eval name as directory or direct task YAML path and requires an eval-name argument. | Eval routing should accept both task paths and named eval directories with explicit errors. |
| `benchmarks/evals/read_over_cat/task.yml` | Validates agents use read tools instead of shell `cat`, catches cat pipes/redirection and multiple-file cat anti-patterns. | File-reading evals should measure correct tool selection, not only task completion. |
| `benchmarks/evals/patch_exact_match/task.yml` | Validates patch tool use, no missing operation errors, no text mismatch failures, and exact search text. | Patch evals should prove exact-match editing behavior. |
| `benchmarks/evals/parallel_tool_calls/task.yml` | Validates at least one assistant message includes multiple tool calls. | Parallel tool-call behavior should be measured directly. |
| `scripts/benchmark.sh` | Builds Forge, runs a command ten times, reports avg/min/max, and fails when average exceeds an optional threshold. | Performance thresholds belong to promotion evidence, not normal coding. |
| `scripts/list-all-porcelain.sh` | Runs porcelain list commands, reports runtimes, and documents which list outputs contain `$ID` columns. | Porcelain command inventory supports shell/completion semantics and should be receipt-backed. |

Governance/evaluation parity requirements extracted:

| Requirement | Target primitive | Priority |
| --- | --- | --- |
| Discover `.forge` commands/skills only from project-local canonical directories | `forge_project_local_governance` | P0 |
| Parse command/skill frontmatter and special command tags without executing command bodies | `forge_project_local_governance` | P0 |
| Keep create-plan planning-only, require checkbox implementation tasks, source citations, and validation script receipts | `forge_project_local_governance` | P0 |
| Capture nextest/devcontainer/tool-description governance fixtures separately from runtime behavior | `forge_project_local_governance` | P1 |
| Treat generated GitHub workflow YAML as generated and avoid hand-editing as source of truth | `forge_ci_release_boundary` | P0 |
| Keep release publication, package repository updates, and GitHub label mutations behind explicit external-action routes | `forge_ci_release_boundary` | P0 |
| Preserve multi-target release matrix and secret-redaction boundaries | `forge_ci_release_boundary` | P0 |
| Separate pure bounty desired-state label rules from mutation application | `forge_ci_release_boundary` | P1 |
| Parse eval names as directories or direct task YAML paths | `forge_benchmark_eval_harness` | P0 |
| Preserve eval logs, timeout/early-exit metadata, validation results, and stripped-ANSI output receipts | `forge_benchmark_eval_harness` | P0 |
| Measure read-tool preference, exact patch use, and parallel tool calls directly from debug request traces | `forge_benchmark_eval_harness` | P0 |
| Keep benchmark/eval execution separate from configured eval inventory and promotion claims | `local_coding_governance_evaluation_guard` | P0 |
