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

If either required field is invalid/empty, the reader rejects that spec.

Optional fields with reader defaults:

1. `workflow_type` (default: `hard_agent_workflow`)
2. `default` (default: `false`)
3. `description` (default: `""`)
4. `final_response_policy` (default: `llm_authored_when_online`)
5. `gate_contract` (default: `tool_menu_interface_v1`)

Unknown extra keys are currently ignored by the reader.

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
3. Ensure `stages` is non-empty and ordered by execution flow.
4. Decide whether it should be default (`default: true`) or non-default.
5. Register it in `046a-workflow-reader.rs`.
6. Run workflow reader regression tests.

Suggested test commands:

1. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_loads_external_specs -- --nocapture`
2. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_enforces_single_default -- --nocapture`
3. `cargo test --manifest-path core/layer0/ops/Cargo.toml --lib workflow_reader_sources_current_workflows_from_json_specs -- --nocapture`

## Policy Guardrail

Workflow definitions for assistant response flow must remain JSON specs.
Do not introduce new inline Rust-authored workflow definitions except fail-closed fallback definitions explicitly used for reader-error containment.

## Capability vs Workflow Boundary (Required)

Use this split consistently:

1. Raw system capability/mechanics belong in Rust authority paths.
2. Workflow structure belongs in JSON workflow specs.
3. If a feature is executable authority or kernel/runtime truth, implement it in Rust and reference it from workflow JSON.
4. If a feature is sequencing/flow shape only, implement it as a workflow JSON update.
