# Coding Level 1 Reference Runtime Trace Observations

Date: 2026-05-20

## Trace artifact

Ignored local trace output:

`references/coding-agent-systems/level1_reference_runtime_trace_observations.json`

Ignored local harness:

`references/coding-agent-systems/runtime_trace_harness/level1_reference_runtime_trace.py`

The reference tree is intentionally ignored by git. This document is the tracked
summary of the runtime trace.

## Shared task

Level 1 task:

`Create one file named hello.py containing a greet(name) function.`

Success signal:

`hello.py` exists and contains `def greet(`.

## Run summary

The trace covered 10 local reference systems.

| System | Execution kind | Level 1 mutation | Mutation receipts | Latency |
| --- | --- | ---: | ---: | ---: |
| mini-swe-agent | provider-free agent loop | yes | 1 | 195 ms |
| swe-agent | direct tool surface | yes | 1 | 49 ms |
| swe-rex | metadata surface | no | 0 | 33 ms |
| aider | metadata surface | no | 0 | 199 ms |
| openhands | metadata surface | no | 0 | 38 ms |
| cline | metadata surface | no | 0 | 32 ms |
| continue | metadata surface | no | 0 | 57 ms |
| goose | metadata surface | no | 0 | 0 ms |
| roo-code | metadata surface | no | 0 | 81 ms |
| forgecode | metadata surface | no | 0 | 83 ms |

No systems were blocked after harness setup fixes.

## What fired

`mini-swe-agent` fired a small provider-free loop:

- `DefaultAgent`
- deterministic model output
- local environment command execution
- trajectory save
- file mutation

This is the cleanest reference for an inspectable minimal agent loop. It still
has an agent loop, but it can run without a live provider.

`swe-agent` fired a direct editor primitive:

- `str_replace_editor create`
- temp registry file via `SWE_AGENT_ENV_FILE`
- file mutation

This is the cleanest reference for the Level 1 fast path. It does not need to
boot a full model loop when the task can be satisfied by a direct mutation
primitive.

`swe-rex`, `aider`, `openhands`, `cline`, `continue`, `goose`, `roo-code`, and
`forgecode` exposed useful runtime or tooling surfaces, but the trace did not
isolate a provider-free create-one-file path for them in this checkout.

ForgeCode is still useful for higher-level coding behavior references. Its
available artifacts emphasize command execution, validation callbacks, command
generation, retry reflection, and loop guards. This Level 1 probe did not use it
as a direct writer.

## Primitive lessons for Infring

Level 1 should not require a cold full-provider loop when the task is already a
complete single-file mutation request.

The primitive should be general, not eval-specific:

`single_mutation_execution`

Inputs:

- explicit target path or safe new-file location
- desired file content or enough localized instructions to produce it
- permission receipt for write/patch

Outputs:

- mutation receipt
- changed-file summary
- optional validation receipt
- structured failure if target/content/path is ambiguous

The gate must not hardcode `hello.py`, `greet`, or any level name. Those belong
only in eval fixtures.

## Why our native Level 1 is slower

Recent native Infring Level 1 passed end-to-end 20/20, but took roughly 21 to
123 seconds per run because each attempt used:

- cold `cargo run -p xtask`
- `ollama run`
- cloud Kimi model call
- full workflow preamble
- model-mediated tool selection

The reference traces show that mature systems have a faster lower lane:

- direct editor primitive
- local deterministic/runtime action loop
- warm session or persistent runtime
- trace/trajectory recording independent of final chat synthesis

## Recommended runtime direction

Keep the coding workflow primitive-first:

1. Add a general direct mutation lane for fully specified single-slice edits.
2. Keep the model loop for ambiguous, multi-file, existing-project, or repair tasks.
3. Preserve receipts and traces for both paths using one schema.
4. Treat metadata-only reference probes as pattern evidence, not task success.
5. Compare future Infring Level 1 against the direct-mutation latency floor, not only pass rate.

