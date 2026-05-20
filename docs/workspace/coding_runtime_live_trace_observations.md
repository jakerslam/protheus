# Coding Runtime Live Trace Observations

Status: ForgeCode-inclusive provider-free trace pass complete

Source trace:

```text
references/coding-agent-systems/runtime_live_trace_observations.json
```

Trace schema:

```text
references/coding-agent-systems/runtime_live_trace_schema.json
```

Harness:

```text
references/coding-agent-systems/runtime_trace_harness/live_trace_coding_runtime.py
```

## Run result

The live trace harness ran provider-free probes against local downloaded coding-agent repos.

Result:

- systems probed: 8
- probes run: 12
- passing probes: 9
- structured setup/blocker probes: 3

Passing probes:

- mini-SWE-agent
- mini-SWE-agent shared task loop
- SWE-agent setup diagnostics
- SWE-ReX
- Aider setup diagnostics
- OpenHands
- Cline
- Continue
- ForgeCode runtime artifact surface

Structured blocker probes:

- SWE-agent tool surface: local search/edit execution did not complete successfully in this checkout, but setup diagnostics found the scripts.
- Aider import surface: direct module import failed in this checkout, but setup diagnostics captured module-level status.
- OpenHands deep surfaces: base package import works, but deeper action/observation/state probe did not yet reach enough importable surfaces.

These blockers are useful runtime evidence: they tell us which reference systems need dependency/setup work before deeper live behavioral replay.

## Live runtime signals captured

### mini-SWE-agent

Probe:

```text
mini_swe_agent_fake_loop
```

Observed event sequence:

```text
command_start
command_result
model_turn_sequence
trajectory_saved
tool_action_observed
```

Runtime implication:

The compact loop can be exercised provider-free with a fake model and fake environment:

```text
model query -> action execution -> observation message -> trajectory persistence -> terminal exit
```

This strongly supports Infring primitives:

- `step_budgeted_trajectory_runtime`
- `tool_action_execution`
- `incremental_receipt_journal`
- `bounded_repair_loop`

Additional probe:

```text
mini_swe_agent_shared_task_loop
```

Observed event sequence:

```text
command_start
command_result
shared_task_trace
shared_task_trace
shared_task_trace
shared_task_trace
```

Shared task traces covered:

- create one file
- patch existing behavior after reading context
- run validation and capture command result
- emit terminal blocker after failed validation

Runtime implication:

The same compact loop can express the four task classes we need for coding Level 1 and early Level 2:

```text
create -> patch -> validate -> block/repair signal
```

This is the strongest live evidence so far that Infring should build a small deterministic coding spine rather than keep expanding the large workflow prompt.

### SWE-ReX

Probe:

```text
swe_rex_command_receipt
```

Observed event sequence:

```text
command_start
command_result
runtime_failure_types_loaded
```

Runtime implication:

Runtime command failure classes are locally importable, supporting explicit command receipt/failure envelopes.

This supports Infring primitives:

- `validation_runner`
- `command_receipt_envelope`
- `failure_diagnosis`

### OpenHands

Probe:

```text
openhands_import_surface
```

Observed event sequence:

```text
command_start
command_result
module_imported
```

Runtime implication:

The local OpenHands package surface is importable enough for deeper future probes against action/observation/state modules.

Additional deep-surface probe:

```text
openhands_deep_surfaces
```

Result:

```text
structured blocker: openhands_deep_imports_failed
```

Runtime implication:

The current checkout can load the base package, but deeper action/observation/state import paths need updated path discovery or dependency setup before full live event replay.

This supports the next live-trace stage:

- event/state replay probe
- stuck-detector probe
- runtime boundary probe

### Cline

Probe:

```text
cline_node_surface
```

Observed event sequence:

```text
command_start
command_result
package_surface_loaded
```

Runtime implication:

The local Cline package metadata is executable/readable through Node, so later probes can target package scripts or focused TypeScript source surfaces.

This supports Infring primitives:

- `tool_lifecycle_projection`
- `permission_policy_gate`
- `patch_executor`

### Continue

Probe:

```text
continue_node_surface
```

Observed event sequence:

```text
command_start
command_result
package_surface_loaded
```

Runtime implication:

The local Continue package metadata is executable/readable through Node, so context/tool provider surfaces can be probed further.

This supports Infring primitives:

- `context_provider_registry`
- `tool_provider_abstraction`

### ForgeCode

Probe:

```text
forgecode_runtime_artifact_surface
```

Observed event sequence:

```text
forgecode_artifact_matrix
command_start
command_result
forgecode_schema_loaded
command_start
command_result
forgecode_cargo_metadata_surface
forgecode_behavior_flags
```

Runtime implication:

ForgeCode is now included as a first-class top-benchmark reference. The provider-free probe confirms its local runtime artifacts are present and parseable, including the Rust workspace metadata, configuration schema, benchmark task executor, validation processor, templated command generator, and control templates for retry reflection, doom-loop interruption, and pending todo completion.

Confirmed behavior flags:

- tool retry template exists
- partial tool-error reflection exists
- doom-loop interruption exists
- pending-todo finalization gate exists
- task timeout support exists
- early-exit validation support exists
- structured validation result support exists
- templated command/context generation exists
- runtime limit configuration exists

This supports Infring primitives:

- `tool_retry_reflection`
- `doom_loop_interrupt`
- `pending_todo_completion_gate`
- `benchmark_validation_harness`
- `configurable_runtime_limits`

## What v2 changes in the model

The live trace pass confirms that the coding model should be event-first:

```text
runtime state
-> model/action decision
-> tool/runtime execution
-> observation/receipt
-> persisted trajectory
-> gate decision
```

The strongest live confirmation is mini-SWE-agent:

- model turns are explicit,
- actions are executed by an environment,
- observations are appended,
- trajectory is saved,
- terminal state is explicit.

The shared-task mini-SWE probe adds a stronger point:

- create-file, patch-existing, validation, and blocker traces can all be represented in the same event loop.

This means Infring should not rely on final-output parsing to infer whether a task progressed. It should record and gate state transitions directly.

## Remaining live-trace gaps

The v2 model is stronger than v1, but still not complete for full behavioral assimilation.

Missing:

- real LLM-backed runs across references,
- reference traces on the same four shared tasks,
- deeper Aider edit/runtime probe after resolving import/dependency blockers,
- deeper SWE-agent executable tool probe after resolving script/dependency blockers,
- OpenHands action/observation/stuck live probe,
- timing baselines against reference systems.

## Next live-trace stage

The next stage should add:

1. `mini-swe-agent shared task replay`
   Use fake and then real configured model if available.

2. `OpenHands state probe`
   Import and exercise action/observation/state classes without a full sandbox.

3. `SWE-agent tool setup probe`
   Run tool install scripts in a temporary workspace or call scripts through their intended environment.

4. `Aider edit-format probe`
   Import/apply edit utilities with local dependencies, or record exact missing dependency blockers.

5. `Infring trace comparator`
   Compare Infring native journals to the live event order captured here.
