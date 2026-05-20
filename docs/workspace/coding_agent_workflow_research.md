# Coding Agent Workflow Research

## Purpose

This document consolidates the current coding-workflow problems, the capabilities needed to pass the native leveled evals, and the useful patterns found in local reference material for ForgeCode, OpenHands, Aider, and the current Codex/Infring workflow docs.

The goal is not to copy another system byte-for-byte. The goal is to identify primitive, measurable mechanics that make Infring-native agents reliable at local software work.

## Research boundaries

This pass used local repository material only.

| Reference | Material inspected | Confidence |
| --- | --- | --- |
| ForgeCode | Full assimilation notes plus direct source-side tool descriptions/templates under `local/workspace/assimilations/ForgeCode-Assimilation/target-repo` | High |
| OpenHands | Local intake/reference docs under `local/workspace/research/openhands-intake` | Medium |
| Aider | Local intake/reference docs under `local/workspace/research/aider-intake` | Medium-low |
| Codex/Infring | Local workflow/eval/enforcer docs under `docs/workspace` | High |

The OpenHands and Aider intake docs refer to isolated vendor clones, but the expected local vendor paths were not present at the attempted paths during this pass. Findings from those systems should therefore be treated as intake-backed patterns until a direct source pass confirms exact implementation details.

## Current situation

The native coding workflow has become a real wedge, but the recent debugging has shown that reliability is fragile. We are getting useful code out of the system, but small global controller changes can regress simpler levels.

The broad pattern:

1. Level-specific patches improved one case.
2. Those patches leaked into lower levels or adjacent task shapes.
3. The workflow became more stage-heavy.
4. Simple tasks became slower or started failing.
5. Existing-project tasks still had intermittent gaps around test mutation, stale patches, preservation, and validation repair.

The core lesson is that we need fewer ad hoc global guardrails and more small, typed coding primitives with clear activation gates.

## Current goals

| Goal | Target |
| --- | --- |
| Native execution | Infring-native agents should perform the work, not Codex worker agents simulating the flow. |
| Useful-work proof | Successful runs need receipts for context discovery, reads, mutations, validation, and final answer claims. |
| Reliability | Reach 19/20 or better on each promoted level before moving upward. |
| Speed | Simple create/write tasks should complete on a fast lane, not pay the cost of full project operation. |
| Scalability | Higher levels should use roadmap, checkpoint, memory, and bounded continuation without contaminating small tasks. |
| Measurability | Failures should return actionable reasons to the higher workflow and ultimately to the user. |

## Leveled capability targets

| Level band | Intended capability | Needed proof |
| --- | --- | --- |
| Level 1 | Simple local code creation or tiny one-file work | File write receipt, runnable/simple semantic output, fast completion |
| Level 2 | Existing small package extension while preserving baseline behavior | Relevant reads, source mutation, test mutation when requested, validation receipt, preservation evidence |
| Levels 3-5 | Multi-file vertical slices and validation repair | Multi-file reads/writes, expected symbols, command validation, repair from failures |
| Levels 6-9 | More complex native project work with richer constraints | Discovery, bounded implementation loop, structured failures, reasoning/status summaries, completion evidence |
| Levels 10-12 | Long-running project-operator behavior | Roadmap, checkpoints, memory/context persistence, safe continuation, export/import/diff style evidence |

The promotion target remains 19/20 Infring-native success, with timing recorded as part of the eval result.

## Current failure classes

| Failure class | Symptom | Likely cause |
| --- | --- | --- |
| Downward regression | Level 1 or Level 2 fails after Level 4+ patches | Global controller logic is doing level-specific work without task-class gating |
| Slow simple tasks | Hello-world style work takes minutes | Full project discovery/repair machinery activates when a direct create path is enough |
| Missing test mutation | Source changes are correct, validation may pass, but no test file mutation receipt exists | Test requirement closure is treated as a prompt hint instead of a workflow primitive |
| Preservation drift | Existing public behavior changes unexpectedly | Agent rewrites or edits without a baseline-preservation contract tied to relevant reads and semantic probes |
| Stale patch loops | Patch old text is not found, then agent repeatedly reads or retries similar invalid edits | No strong stale-patch failure classifier or loop breaker |
| Read loops | Agent keeps reading after enough context or after successful validation | Missing stateful stuck/repetition detection and stage-level eligibility |
| Tool menu mismatch | Runtime still accepts or routes calls that the current stage should not allow | Tool eligibility is not enforced tightly enough at runtime boundary |
| Write vs patch tension | Full-file writes fix some additive tasks but break preservation tasks | Existing-file edit policy is not encoded as a dedicated primitive with snapshots and overwrite gates |
| Validation timing | Auto-validation runs before required test mutation, causing false completion failure | Validation is not sequenced behind requirement closure |
| Harness contamination | Runs fail from disk/tooling/environment issues instead of workflow behavior | Eval isolation and disk/toolchain readiness are not strong enough |

## Needed capabilities

| Capability | Why it matters |
| --- | --- |
| Task-lane classifier | Choose fast create, existing-project patch, validation repair, or long-run operator before activating heavyweight stages |
| Repo context assessment | Decide whether the task is a new project, existing project, small package, or larger codebase |
| Bounded file discovery | Find relevant files without broad, repeated, or irrelevant reads |
| File edit primitive | Provide read, patch, write, multi-patch, snapshot, and receipt semantics as one coherent local coding substrate |
| Test requirement closure | If tests are required, ensure test mutation happens before validation and final success |
| Preservation contract | Explicitly protect public APIs, baseline outputs, imports, and existing behavior in existing-project tasks |
| Validation command runner | Run commands with structured stdout, stderr, exit code, cwd, and timing receipts |
| Validation repair loop | Classify failures, attempt bounded repair, and stop with actionable blocker reasons |
| Stuck detector | Detect repeated actions, repeated observations, repeated errors, and stale patch loops |
| Tool eligibility enforcement | Runtime should fail closed when a stage emits a disallowed tool call |
| Final-answer receipt binding | Final response should only claim changes and validation supported by receipts |
| Eval isolation | Ensure failures are from agent behavior, not dirty workspaces, disk exhaustion, or missing harness binaries |

## Reference findings and solutions

### ForgeCode

ForgeCode is the most directly relevant reference for local coding workflow behavior. Its assimilation material identifies a three-lane mental model:

| Lane | Role | Infring mapping |
| --- | --- | --- |
| `sage` | Read-only research and investigation | Repo context assessment and evidence gathering |
| `muse` | Planning, checklist, risks, sequencing | Implementation plan and completion contract |
| `forge` | Concrete implementation and validation | Local edit execution plus validation repair |

Useful ForgeCode mechanics:

| Pattern | Observed source/reference | Infring solution |
| --- | --- | --- |
| Dedicated file tools | `fs_read`, `fs_patch`, `fs_write`, `fs_multi_patch` descriptions | Keep file read/write/patch out of shell and expose a coherent local coding file capability pack |
| Absolute path discipline | File tool descriptions require absolute paths | Native file tools should normalize and reject ambiguous paths consistently |
| Read before edit | Patch/write descriptions require reading existing files first | Existing-file mutation primitive should require read receipt before mutation unless creating a new file |
| Prefer patch over write | `fs_patch` is preferred for existing files; `fs_write` overwrites | Use patch/multi-patch for preservation tasks; reserve write for new files or explicit overwrite with snapshot |
| Atomic multi-patch | `fs_multi_patch` applies all-or-nothing edits in one file | Add or strengthen a native multi-patch primitive to reduce drift from multiple same-file edits |
| Exact-match patch errors | Patch fails on no-match or non-unique old text | Convert these errors into classified repair prompts instead of repeated blind reads |
| Tool failure budget | Assimilation notes include budget and recovery behavior | Add bounded repair attempts per failure class and return structured blocker after budget |
| Doom-loop reminder | ForgeCode has a repetitive-call reminder template | Add generic stuck detector for repeated read/patch/command patterns |
| Tool retry reflection | Retry template asks model to analyze the error and adjust | Feed classified tool errors back as compact, action-specific repair instructions |
| Shell is not file ops | Shell description says terminal operations only | Keep `command_run` separate from file tools and prevent shell from becoming a write bypass |

Recommended ForgeCode-derived primitive:

`local_code_edit_execution`

Responsibilities:

1. Classify create vs existing-file mutation.
2. Require read receipt before existing-file mutation.
3. Prefer patch or multi-patch for existing files.
4. Allow write for new files and explicit overwrite only.
5. Snapshot before overwrite.
6. Emit line/hash/path receipts for reads and mutations.
7. Classify no-match, multi-match, stale context, permission, binary, size, and path errors.
8. Return compact repair instructions to the workflow.

This should replace much of the current ad hoc controller patching around read/write/patch behavior.

### OpenHands

OpenHands is most useful for controller reliability, stuck detection, runtime boundaries, and state handling.

Useful OpenHands patterns from the intake docs:

| Pattern | Why it matters | Infring solution |
| --- | --- | --- |
| Event-sourced step eligibility | Prevents tools from firing in invalid states | Model each coding step as eligible only when prerequisite receipts exist |
| Typed exception/status mapping | Makes failures actionable | Convert tool and validation failures into stable workflow failure classes |
| Pending-action reset evidence | Avoids ambiguous in-flight state | Ensure failed or interrupted actions produce terminal observations before next step |
| State persistence/resume | Enables long-run coding | Store roadmap, checkpoint, memory, and receipt refs for Levels 10-12 |
| Filtered history/state tracker | Keeps context bounded | Summarize prior actions and retain only relevant tool receipts in prompt context |
| Stuck detector | Detects repeated actions/errors/monologues/context issues | Add a native coding stuck detector before retrying the same action family |
| Runtime mutation lock | Avoids conflicting file mutations | Serialize native file writes/patches per workspace or per file |
| File editor error envelopes | Makes edit failures machine-readable | Standardize file tool errors for no-match, multi-match, stale read, overwrite denied |
| Bounded file navigation/search | Avoids read/search explosion | Add discovery budgets and require narrowing after broad matches |

Recommended OpenHands-derived primitive:

`coding_loop_supervisor`

Responsibilities:

1. Track step eligibility from receipts.
2. Detect repeated equivalent tool calls.
3. Detect repeated equivalent errors.
4. Decide whether to retry, change strategy, escalate, or stop.
5. Emit a structured blocker reason to the parent workflow when progress stalls.

This should be general and not tied to any specific level.

### Aider

The Aider intake available locally is narrower, but it points to a useful discovery hardening pattern.

Useful Aider pattern:

| Pattern | Why it matters | Infring solution |
| --- | --- | --- |
| Repo-filtered glob/file calling | Avoids irrelevant file discovery and hidden/vendor noise | Build discovery around tracked files, ignore rules, and repo-relative narrowing |

Recommended Aider-derived primitive:

`repo_context_assessment`

Responsibilities:

1. Detect repo root and project shape.
2. Use tracked-file or ignore-aware filtering where available.
3. Identify likely source/test/config files.
4. Return a small candidate file set and confidence score.
5. Let the parent workflow ask for clarification if confidence is too low.

This is especially important for existing-project tasks and should not run for trivial new-file tasks unless the task implies an existing codebase.

### Codex/Infring docs

The current docs already contain several important constraints we should lean into instead of fighting.

Useful current-doc patterns:

| Pattern | Source | Infring solution |
| --- | --- | --- |
| Task classification | `codex_enforcer.md` | Add a native coding task-lane classifier and use it before expensive workflow steps |
| Stop-loss behavior | `codex_enforcer.md` | Cap repair loops and return actionable blocker summaries |
| Workflow CD as typed program | `codex_enforcer.md` | Move behavior into workflow/tool contracts, not hardcoded prompt phrases in Rust |
| No fake success | `native_coding_useful_work_eval_v1.md` | Require receipts for mutation, validation, and final claims |
| Timing metrics | `native_coding_useful_work_eval_v1.md` | Keep completion time in eval output for all levels |
| High-level memory/checkpoint targets | `coding_workflow_eval_results.md` | Keep Levels 10-12 under the higher-level coding operator, not the primitive edit loop |

Recommended Codex/Infring-derived primitive:

`coding_task_lane_classifier`

Responsibilities:

1. Classify task as `new_file_fast_path`, `existing_project_patch`, `validation_repair`, `multi_file_slice`, or `long_run_project_operator`.
2. Decide whether repo discovery is required.
3. Decide whether tests are required by prompt or eval contract.
4. Decide whether preservation evidence is required.
5. Decide whether memory/checkpoint machinery is required.

This is the main guard against Level 4+ machinery poisoning Level 1 and Level 2.

## Proposed workflow architecture

The current master coding workflow should become a composite that routes through small primitives rather than containing all behavior directly.

```text
coding_task_lane_classifier
  -> new_file_fast_path
  -> repo_context_assessment
      -> implementation_plan
      -> local_code_edit_execution
      -> test_requirement_closure
      -> validation_repair_loop
      -> final_receipt_synthesis
  -> long_run_project_operator
      -> roadmap/checkpoint/memory
      -> repeated bounded slices using the same lower primitives
```

Primitive ownership:

| Primitive | Workflow level | Purpose |
| --- | --- | --- |
| `coding_task_lane_classifier` | 0 | Choose the smallest safe lane |
| `repo_context_assessment` | 0 | Gather bounded project context |
| `local_code_edit_execution` | 0 | Perform local reads/writes/patches with receipts |
| `test_requirement_closure` | 0 | Ensure required tests are created or updated before validation |
| `validation_repair_loop` | 0 or 1 | Run validation, classify failure, repair within budget |
| `coding_loop_supervisor` | 0 | Detect stuck/retry/loop conditions |
| `coding_project_operator` | 1+ | Compose primitives for normal coding tasks |
| `long_run_project_operator` | 2+ | Add roadmap, checkpoint, memory, continuation |

The important design principle:

Do not make the primitive edit loop know about all levels. Make it excellent at one bounded slice. Let higher-level workflows decide how many slices to run and when to stop.

## Specific fixes suggested by this research

### P0: Restore a true fast lane

Implementation status:

Implemented as `new_file_fast_path` in the native prompt/runtime path. The runtime now classifies probable micro direct-write tasks, skips bootstrap discovery and staged-controller blocking for that lane, and emits the selected `coding_task_lane` in native loop metadata.

Problem:

Level 1 regressed because simple tasks were exposed to existing-project guardrails.

Solution:

Add or strengthen `new_file_fast_path` activation:

1. If the task is clearly creating a new small file or self-contained snippet, skip repo discovery.
2. Allow direct file write.
3. Require only write receipt and optional lightweight semantic check.
4. Do not activate preservation, test mutation, roadmap, memory, or validation repair unless requested.

### P0: Move level-specific guardrails out of global controller logic

Implementation status:

Partially implemented with a native coding task-lane classifier. Current lanes are `new_file_fast_path`, `existing_project_patch`, `validation_repair`, `multi_file_slice`, `long_run_project_operator`, `implementation_slice`, and `general_native_tool_task`. The initial prompt now exposes the lane, and native loop metadata records it for eval/debugging.

Problem:

Global controller hints and runtime blocks are accumulating. They can fix one benchmark while regressing another.

Solution:

Move behavior into typed workflow contracts and primitives:

1. The task lane classifier decides which contracts apply.
2. Tool eligibility is derived from the active contract.
3. Runtime enforces the contract generically.
4. Failure repair is based on failure class, not benchmark-specific text.

### P0: Create a dedicated test requirement closure primitive

Implementation status:

Partially implemented in `core/layer2/agent_surface/src/agent.rs` and `core/layer2/agent_surface/src/native_prompt_policy.rs`. The staged runtime controller now treats requested tests as a closure gate after product/source mutation: validation, handoff, memory closure, and non-test mutations are blocked until a successful test file write or patch receipt exists. A small amount of test-path discovery remains allowed when no test path has been observed. The controller also now distinguishes product/source mutation from test mutation, so writing only tests no longer satisfies the product/source stage.

Problem:

The agent often produces correct source code but fails because no test file mutation receipt exists.

Solution:

Add `test_requirement_closure`:

1. Activate only when prompt/eval requires tests.
2. Inspect source package/import style from existing reads.
3. Require test file write or patch before validation.
4. If validation passes before test mutation, do not finalize; request test closure.
5. Emit `missing_test_mutation`, `test_import_error`, or `test_scope_unclear` as structured failure classes.

### P0: Add stale patch and repeat-action repair

Problem:

Patch failures can lead to repeated reads or repeated invalid patches.

Solution:

Use ForgeCode and OpenHands patterns:

1. On patch no-match, require either fresh targeted read of the exact file region or switch to multi-patch/write if safe.
2. On repeated same no-match, stop retrying and emit stale context blocker.
3. On repeated read of same file without new evidence, block the read and ask for an edit or escalation.
4. On repeated validation failure with same stderr signature, switch strategy or stop.

### P0: Implement preservation as a contract, not a numeric-token hack

Implementation status:

Partially implemented. The staged product/source gate now distinguishes a generic product mutation from the product source surface required by the prompt. If evidence gaps still include a missing product/source changed path, such as the module that owns a preserved public API, the runtime keeps the task in the product/source stage and blocks tests, validation, handoff, and memory closure until that source surface is updated. The gate also scans the local project for preserved API definitions and includes missing source paths in live staged-block receipts so the next tool call can patch the right module instead of creating only sibling modules.

The preservation guard now compares preserved Python API behavior signatures, especially return/raise/yield lines, rather than only numeric tokens. It also applies to patch-style edits when the proposed result can be simulated. This is intended to prevent preserved behavior from changing return shape or control flow while still allowing adjacent additive APIs and harmless formatting changes.

Problem:

Preservation drift is real, but brittle guards around numeric tokens and full-file writes are too specific.

Solution:

For existing-project tasks:

1. Record baseline public symbols and representative behavior from relevant source/tests.
2. Prefer patch/multi-patch over full-file write.
3. Require final evidence that protected symbols still exist.
4. If semantic probe is available, run it after validation.
5. If full-file overwrite is needed, require snapshot and explicit overwrite eligibility.

### P1: Add repo-filtered discovery

Problem:

Agents need to infer relevant files from natural user requests without hardcoded "read X" prompts.

Solution:

Use Aider-style repo filtering plus ForgeCode-style semantic/direct search hierarchy:

1. Detect repo root.
2. Prefer tracked files and ignore-aware walking.
3. Narrow source/test/config candidates by task terms.
4. Return small candidate sets with confidence.
5. If confidence is low, ask the user instead of broad-scanning.

### P1: Strengthen validation repair classification

Implementation status:

Partially implemented. The staged validation controller now allows a small amount of targeted source/test inspection after a failed validation command before blocking further read-only loops. The coding workflow CD also now names parse/input-shape mismatch, object attribute mismatch, and API-placement failures as implementation/export repair cases.

Problem:

Validation failures are not all the same. Some need import repair, some need behavior repair, some need test correction, some need user input.

Solution:

Classify validation failures:

| Failure class | Next action |
| --- | --- |
| Import/module path error | Inspect package layout and repair import path |
| Assertion mismatch | Compare expected behavior to changed source and repair source or test |
| Missing dependency/tool | Structured blocker, do not fake success |
| Syntax/type error | Targeted source repair |
| Same failure repeated | Stop or escalate with receipt summary |

### P1: Improve harness isolation

Problem:

Some failures are infrastructure contamination, not workflow behavior.

Solution:

1. Preflight workspace free space and required runner availability.
2. Use isolated temp workspaces per run.
3. Record infrastructure failure separately from agent failure.
4. Do not count invalid harness failures against workflow quality.

### P2: Keep long-run features behind the high-level operator

Problem:

Memory, roadmap, checkpointing, and project persistence are valuable but expensive.

Solution:

Only activate them for:

1. Long-running tasks.
2. Existing codebase evolution.
3. Multi-checkpoint work.
4. Explicit continuation/resume tasks.

Do not activate them for Level 1 or simple Level 2 tasks.

## Proposed eval changes

| Eval improvement | Purpose |
| --- | --- |
| Record task lane selected | Detect whether the classifier is choosing the right path |
| Record primitive activations | See when heavyweight primitives leak into simple tasks |
| Separate infra failures | Avoid patching workflow for disk/toolchain problems |
| Record first blocker class | Make failures actionable |
| Record repeated tool signatures | Identify stuck loops |
| Record timing by stage | Find slow stages without guessing |
| Require receipt-bound final answer | Prevent overclaiming |

## Solution map by failure

| Current failure | Best reference | Most primitive fix |
| --- | --- | --- |
| Level 1 slow/failing | Codex task classification | `new_file_fast_path` |
| Missing test mutation | Native eval contract plus ForgeCode staged tooling | `test_requirement_closure` |
| Patch no-match loops | ForgeCode retry/reflection plus OpenHands stuck detector | `coding_loop_supervisor` with stale patch class |
| Preservation drift | ForgeCode patch-over-write plus Codex no-fake-success | Preservation contract in existing-project lane |
| Repeated reads | OpenHands stuck detector | Repeated action detection |
| Over-broad discovery | Aider tracked-file filtering | `repo_context_assessment` |
| Validation failure repair | ForgeCode tool retry plus OpenHands typed status | `validation_repair_loop` |
| Controller bloat | Codex CD typed-program policy | Move behavior into primitives/workflow contracts |

## Recommended implementation sequence

1. Add `coding_task_lane_classifier` and make all strict machinery conditional on its result.
2. Restore Level 1 fast lane and rerun Level 1 five times.
3. Add `test_requirement_closure` for test-required tasks.
4. Add stale-patch and repeated-action detection to the coding loop supervisor.
5. Convert current preservation hacks into an existing-project preservation contract.
6. Add repo-filtered discovery for existing-project tasks.
7. Harden validation repair classifications.
8. Improve harness isolation and timing reporting.
9. Rerun levels from 1 upward, five at a time, patching only general primitives.
10. Promote only after 19/20 native runs at the level.

## What not to do next

Avoid these paths unless there is a strong reason:

| Anti-pattern | Why to avoid |
| --- | --- |
| More benchmark-specific prompt text | It can improve one level while hiding a general weakness |
| More global controller guards | They regress simpler lanes |
| Forcing discovery for all tasks | It makes trivial work slow and brittle |
| Full-file overwrite bans everywhere | Some additive tasks legitimately benefit from simple file writes |
| Counting receipts only | Receipts prove action, not task satisfaction |
| Moving long-run memory into primitive edit flow | It makes simple tasks pay long-run costs |

## Bottom line

The main problem is not that the coding workflow lacks enough rules. It has too many rules in the wrong place.

The path forward is to make the workflow more modular:

1. Classify the coding task.
2. Activate only the smallest safe lane.
3. Use ForgeCode-style local file primitives for edits.
4. Use OpenHands-style loop supervision for retries and stuck states.
5. Use Aider-style bounded repo discovery when existing context is needed.
6. Keep Codex/Infring no-fake-success and CD-boundary rules as the enforcement layer.

If we do that, Level 1 should become fast and boring again, Level 2 should stop failing on missing test receipts, and higher levels should become easier to debug because failures will map to explicit primitives instead of a growing blob of controller behavior.
