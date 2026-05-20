# Coding Runtime Reference Observations

Status: ForgeCode-inclusive sensor pass complete

Source trace:

```text
references/coding-agent-systems/runtime_trace_observations.json
```

Trace schema:

```text
references/coding-agent-systems/runtime_trace_schema.json
```

Harness:

```text
references/coding-agent-systems/runtime_trace_harness/trace_coding_runtime.py
```

## Run result

The reference sensor pass scanned the local downloaded coding-agent corpus, including the local ForgeCode assimilation target repo.

Result:

- systems scanned: 8
- observations captured: 28
- systems with observations: 8/8

Systems covered:

- mini-SWE-agent
- SWE-agent
- SWE-ReX
- Aider
- OpenHands
- Cline
- Continue
- ForgeCode

## What the references agree on

The successful coding systems converge on a runtime loop, not a prompt pile:

```text
state
-> allowed action
-> tool/runtime execution
-> observation/receipt
-> progress gate
-> repair or finalization
```

The common primitives are:

- `task_contract`
- `context_pack_builder`
- `implementation_entry_gate`
- `file_mutation_executor`
- `validation_runner`
- `public_interface_verifier`
- `bounded_repair_loop`
- `tool_retry_reflection`
- `doom_loop_interrupt`
- `incremental_receipt_journal`
- `step_budgeted_trajectory_runtime`
- `final_receipt_synthesis`

## System-specific evidence

### mini-SWE-agent

Observed primitives:

- `step_budgeted_trajectory_runtime`
- `tool_action_execution`
- `incremental_trajectory_persistence`

Runtime lesson:

The loop is small and explicit: query the model, execute actions, append observations, save trajectory, check limits/exit.

### SWE-agent

Observed primitives:

- `workspace_discovery_tools`
- `exact_file_edit_contract`
- `terminal_submit_boundary`

Runtime lesson:

Search, edit, and submit are separate tool surfaces. Finalization is not just prose; it is a terminal boundary.

### SWE-ReX

Observed primitives:

- `runtime_execution_boundary`
- `command_receipt_envelope`

Runtime lesson:

Agent logic should not own shell execution directly. Runtime execution returns structured output, exit code, failure reason, and session metadata.

### Aider

Observed primitives:

- `diff_edit_discipline`
- `repo_context_map`
- `validation_repair_signal`

Runtime lesson:

Context is selected under budget before edits. Edits are constrained by formats that can apply cleanly. Validation/lint feedback becomes repair input.

### OpenHands

Observed primitives:

- `event_sourced_controller`
- `stuck_detection`
- `runtime_sandbox_boundary`

Runtime lesson:

Actions, observations, events, state, stuck detection, and runtime boundaries are distinct pieces of the system.

### Cline

Observed primitives:

- `tool_lifecycle_projection`
- `permission_policy_gate`
- `patch_executor`

Runtime lesson:

Tool calls live inside session/turn lifecycle state and run behind policy gates. Patch application is an executor capability with structured errors.

### Continue

Observed primitives:

- `context_provider_registry`
- `tool_provider_abstraction`

Runtime lesson:

Context and tools are provider surfaces, not hidden prompt stuffing.

### ForgeCode

Observed primitives:

- `multi_mode_coding_agent_surface`
- `tool_description_contract`
- `configurable_runtime_limits`
- `benchmark_task_executor`
- `validation_result_contract`
- `templated_command_generation`
- `tool_retry_reflection`
- `doom_loop_interrupt`
- `pending_todo_completion_gate`

Runtime lesson:

ForgeCode treats coding-agent behavior as a configured runtime, not a one-off prompt. The local reference confirms multi-mode operation, registered tool-description discipline, first-class runtime limits, benchmark execution with logs/timeouts/early exit, structured validation results, templated command generation, explicit tool-error reflection, doom-loop interruption, and pending-todo finalization gates.

## Infring implication

The next coding workflow rebuild should not keep adding behavior to the large `coding_project_operator` workflow.

The build order should be:

1. `coding_task_contract`
2. `implementation_entry_gate`
3. `incremental_receipt_journal`
4. `file_mutation_executor`
5. `validation_runner`
6. `public_interface_verifier`
7. `tool_retry_reflection`
8. `doom_loop_interrupt`
9. `bounded_repair_loop`
10. `final_receipt_synthesis`

The high-level workflow should compose those primitives. It should not own the control loop through prompt text.

## Current gap

The first pass is a static reference sensor pass. It gives concrete source-backed mechanics without needing provider credentials or live model calls.

Next sensor stage:

- run provider-free smoke traces where possible,
- add live traces for systems with local test models configured,
- compare actual event order against the Infring native journal,
- make Level 1 pass by implementing the model, not by adding task-specific patches.
