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
| `FC-A05` | Extract validation/repair loop semantics from `forge_ci`, `forge_services`, and `forge_tracker` | active | Initial `validation_command_runner` behavior harness added; repair-loop source pass still pending. |
| `FC-A06` | Implement measurable coding safety behavior in eval ownership | active | `eval_coding_safety_layer` now exercises safe reads, guarded writes, exact-match patches, stale-context rejection, snapshots, hashes, and validation command receipts. |

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

## Runtime behavior harnesses created

| Harness | Owner | Status | Measures |
| --- | --- | --- | --- |
| `coding_safety_layer_lab_behavior_v1` | `orchestration/src/eval_coding_safety_layer.rs` | behavior harness created | Absolute-path reads, line-range hash receipts, guarded writes, snapshot-before-overwrite, exact-match patching, stale-context rejection, and structured validation command receipts. |

Runner:

`cargo run --manifest-path orchestration/Cargo.toml --bin coding_safety_layer_lab_execute`

Neutral master workflow integration:

| Workflow ID | Integration status | Notes |
| --- | --- | --- |
| `local_coding_program_builder` | safety-layer dependency declared | The neutral master workflow now references `local_code_edit_execution` and the safety-layer primitives in its composition ledger. |

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
- `local_coding_program_builder` still uses a lab file materialization harness. It does not yet invoke a live coding agent with repo read/write tools, validation commands, and repair retries.

Next source pass:
- Start with `crates/forge_fs`, `crates/forge_repo`, `crates/forge_walker`, and `crates/forge_services`.
- Extract the minimum contract needed for `local_code_edit_execution`.
