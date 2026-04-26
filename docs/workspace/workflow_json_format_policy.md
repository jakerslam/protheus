# Workflow JSON Format Policy

Status: active  
Scope: assistant response workflows loaded by the control-plane workflow reader

## Purpose

Define one canonical, human/LLM-friendly workflow format so workflow definitions are not embedded ad hoc in Rust logic.

Canonical format: JSON (`*.workflow.json`)

Reader implementation: `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/046a-workflow-reader.rs`

Orchestration template reader implementation:
`surface/orchestration/src/control_plane/templates.rs`

Current workflow spec directory:
`core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/190_route_blocks/agent_scope_full_parts/workflows/`

Orchestration workflow template directory:
`surface/orchestration/src/control_plane/workflows/`

Example template:
`docs/workspace/templates/workflow/workflow_template.workflow.json`

## Format Contract

All workflow specs must be JSON objects.

Required reader-acceptance fields:

1. `name` (string, non-empty after sanitization)
2. `stages` (array of stage strings, at least one non-empty item after sanitization)
3. `workflow_type` (`control_plane_orchestration_workflow` for control-plane workflows)
4. `workflow_role` (`assistant_response_workflow` or `assimilation_workflow_template`)
5. `typed_execution_contract` (object, required for control-plane promotion)

If any required field is invalid/empty, the reader rejects that spec.

Control-plane promotion requires the Rust workflow contract guard to compile each JSON workflow into a typed graph before runtime use. `typed_execution_contract` must include:

1. `gate_kind` (string)
2. `input_kind` (`multiple_choice`, `text_input`, or `multiple_choice_or_text_input`)
3. `allowed_transitions` (`stage_a->stage_b` strings, with terminal transitions allowed)
4. `timeout_ms` (positive integer)
5. `retry_policy.max_retries` and `retry_policy.on_failure`
6. `terminal_states` containing `completed`, `needs_input`, `blocked`, `failed`, and `aborted`
7. `telemetry_streams` containing `workflow_state`, `agent_internal_notes`, `tool_trace`, `eval_trace`, and `final_answer`
8. `tool_family_contracts` containing `workspace`, `web`, `memory`, `agent`, `shell`, and `browser`
9. `visible_chat_policy` set to `llm_final_only_no_system_injection`
10. `run_budgets` with max stages, model turns, tool calls, token budget, and loop-signature detector

Optional fields with reader defaults:

1. `default` (default: `false`)
2. `description` (default: `""`)
3. `final_response_policy` (default: `llm_authored_when_online`)
4. `gate_contract` (default: `tool_menu_interface_v1`)

Unknown extra keys are currently ignored by the reader.

## Workflow Role Rule

The control-plane workflow directory contains both assistant-response workflows and assimilation workflow templates. They must be role-typed so the runtime and guards do not confuse strategy templates with normal chat finalization paths.

Allowed roles:

1. `assistant_response_workflow`: may participate in user-visible assistant response flow and must obey the LLM-final-only policy.
2. `assimilation_workflow_template`: may describe structured assimilation strategy and required signals, but must not be treated as the default user-visible response workflow.

Assimilation workflow templates must declare at least one `subtemplates` row. Each subtemplate must include non-empty `id`, `description`, `required_signals`, `required_gates`, and `source_refs` fields so assimilation can be audited as capability transfer instead of ledger burn-down.

Subtemplate `id` values must be unique within the workflow, no longer than 120 characters, and limited to lowercase ASCII letters, digits, `_`, and `-`. Subtemplate `required_signals`, `required_gates`, and `source_refs` must not contain duplicate values. `source_refs` must be repo-relative or local-assimilation paths under approved roots such as `local/workspace/assimilations/`, `local/workspace/vendor/`, `surface/orchestration/`, `docs/workspace/`, `tests/tooling/`, `core/`, or `adapters/`; absolute paths, URL refs, and `..` traversal are invalid.

Assistant-response workflows must not declare `subtemplates`; if a normal response path needs reusable sequencing, promote it into stages/contracts rather than embedding assimilation doctrine.

## Sanitization + Length Limits

The reader sanitizes/truncates text values via `clean_text(...)`.

Current max lengths:

1. `name`: 80
2. `workflow_type`: 80
3. `description`: 600
4. `stage` item: 120
5. `final_response_policy`: 120
6. `gate_contract`: 80

## Default Workflow Rule

The library must resolve to one default workflow.

Normalization behavior:

1. If no spec is marked `default: true`, the first loaded spec is promoted to default.
2. If multiple specs are marked `default: true`, only the first remains default.
3. If no valid specs load, reader falls back to built-in `workflow_spec_error_v1` (fail-closed).

## Interface-Only Workflow Rule

Assistant chat workflows are tool access interfaces, not tool-picking advisors.

Allowed gate shapes:

1. Multiple choice menu
2. Text input / request payload field
3. Tool execution handoff using the submitted payload
4. Telemetry-only status export
5. LLM-authored final output

Disallowed workflow behavior:

1. Recommending a tool family
2. Inferring whether a tool is needed
3. Classifying the user turn as task/info
4. Choosing a tool automatically
5. Injecting fallback text into the visible chat response
6. Adding "next actions" or system-authored diagnostic prose to the final answer

No-injection invariant:

1. System-authored fallback text must never be inserted into visible chat.
2. Failure/finalization diagnostics go to telemetry, attention queues, or UI diagnostic streams only.
3. Visible chat text is emitted only by the LLM final output stage.
4. `visible_chat_policy` must remain `llm_final_only_no_system_injection`.

Canonical stage vocabulary:

1. `gate_1_need_tool_access_menu`
2. `gate_2_tool_family_menu`
3. `gate_3_tool_menu`
4. `gate_4_request_payload_input`
5. `gate_5_post_tool_menu`
6. `gate_6_llm_final_output`

Direct conversation is represented by `No` at `gate_1_need_tool_access_menu`, not by a separate automatic bypass classifier.

## Registration Rule (Required)

A valid JSON file is not loaded automatically just by existing in the folder.
It must be wired in `046a-workflow-reader.rs`:

1. Add an `include_str!(...)` constant for the file.
2. Add `(source_path, constant)` entry to `WORKFLOW_SPEC_SOURCES`.
3. Ensure the workflow appears in the library catalog tests if applicable.

## Authoring Checklist

Before opening a PR for a new workflow:

1. Start from `docs/workspace/templates/workflow/workflow_template.workflow.json`.
2. Keep `name` unique and versioned (example: `_v1`, `_v2`).
3. Set `workflow_type` to `control_plane_orchestration_workflow`.
4. Set `workflow_role` to either `assistant_response_workflow` or `assimilation_workflow_template`.
5. Ensure `stages` is non-empty and ordered by execution flow.
6. Decide whether it should be default (`default: true`) or non-default.
7. Register it in `046a-workflow-reader.rs` when it belongs to the assistant response reader.
8. Run workflow reader regression tests.

Suggested test commands:

1. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_loads_external_specs -- --nocapture`
2. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_enforces_single_default -- --nocapture`
3. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_sources_current_workflows_from_json_specs -- --nocapture`
4. `cargo test --manifest-path surface/orchestration/Cargo.toml workflow_contract -- --nocapture`
5. `cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin workflow_contract_guard -- --strict=1`

## Policy Guardrail

Workflow definitions for assistant response flow must remain JSON specs.
Do not introduce new inline Rust-authored workflow definitions except fail-closed fallback definitions explicitly used for reader-error containment.

## Capability vs Workflow Boundary (Required)

Use this split consistently:

1. Raw system capability/mechanics belong in Rust authority paths.
2. Workflow structure belongs in JSON workflow specs.
3. If a feature is executable authority or kernel/runtime truth, implement it in Rust and reference it from workflow JSON.
4. If a feature is sequencing/flow shape only, implement it as a workflow JSON update.
