# The Three Operating Laws

Status: hard repo-wide policy.

InfRing should be judged by whether it is useful, reliable, and simple enough to keep improving.

This document is the operating filter for new backlog items, Sentinel recommendations, eval findings, assimilation work, maintenance proposals, and new subsystem proposals.

Informal nickname: `the three commandments`.

## One-Line Rule

Build what is useful. Make it reliable. Keep it simple enough to survive.

## The Three Operating Laws

1. `usability`: the system must help a user or agent complete concrete work.
2. `reliability`: the useful path must keep working under normal pressure and fail with actionable diagnostics when it cannot.
3. `simplicity`: the system must get easier, not harder, to reason about as it grows.

These laws are ordered. A simple system that does nothing is not enough. A usable system that breaks is not enough. A reliable useful system that becomes incomprehensible will eventually collapse.

## Admission Gates

Every non-trivial work item should pass at least one gate:

- `real_work`: directly helps a user or agent complete a concrete task end to end.
- `reliability`: prevents regression, hang, install failure, data loss, boundary leak, or false confidence on a real-work path.
- `simplification`: removes duplication, stale code, unnecessary abstractions, or cognitive load without weakening real-work behavior.

If an item passes none of these gates, it should be parked, archived, or rejected.

Active work that violates all three gates is policy debt, even when it is technically interesting.

## Sacred Workflow

The first workflow the system must make boringly reliable is:

1. Receive a user request, issue, alert, or Sentinel finding.
2. Inspect the smallest useful evidence set.
3. Locate the owning subsystem and source files.
4. Patch narrowly.
5. Run targeted validation when appropriate.
6. Explain the change and residual risk.
7. Update the TODO, issue, or release artifact.

This workflow is more important than adding broad new subsystems. New machinery should either improve this workflow or wait.

## First Canonical Real-Work Loop

Security and correctness alert remediation is the initial canonical loop:

1. Ingest alert details.
2. Identify the source file, line, rule, and failure mode.
3. Patch the smallest safe behavior change.
4. Run targeted validation for the touched path.
5. Commit with a conventional prefix.
6. Record the closure pattern for future alerts.

This loop is useful because it is external, concrete, measurable, and recurring.

## Sentinel and Eval Expectations

Sentinel and eval outputs should be useful inputs to the sacred workflow. A finding should include:

- evidence reference
- recurrence or freshness signal
- likely owner or domain
- root-cause hypothesis
- proposed fix class
- targeted validation suggestion
- TODO or issue candidate wording

Findings without evidence or a next action should remain internal observations instead of becoming active work.

Sentinel must treat repeated violations of this doctrine as system health findings:

- work that grows capability surface without a concrete real-work loop
- reliability paths that fail or hang without bounded diagnostics
- complexity growth that adds representations, ownership ambiguity, or compatibility tails without protecting real work
- recurring TODO, issue, or Sentinel findings that never become patchable tasks

Sentinel findings should label the violated law as `usability`, `reliability`, or `simplicity`.

## Reliability Floor

The system is not meaningfully useful if these paths are unreliable:

- install and repair
- gateway start/status/restart
- request-to-response workflow execution
- alert or issue remediation
- Sentinel observation to actionable finding
- TODO or issue lifecycle
- commit and release hygiene

These paths should outrank speculative capability work.

Canonical reliability floor policy:

- [reliability_floor_policy.md](/Users/jay/.openclaw/workspace/docs/workspace/reliability_floor_policy.md)

Canonical CodeQL real-work loop:

- [CODEQL_REMEDIATION_WORKFLOW.md](/Users/jay/.openclaw/workspace/docs/workspace/process/CODEQL_REMEDIATION_WORKFLOW.md)

## Simplification Rule

Simplification is not cleanup theater. It should reduce one of:

- duplicate source-of-truth representations
- abandoned compatibility paths
- confusing ownership boundaries
- generated or stale artifacts that look authoritative
- large files that hide unrelated responsibilities
- test or validation scaffolding that no longer proves current behavior

If simplification cannot name the real-work path it protects, it should be lower priority.

## TODO Metadata

TODO items should prefer these optional fields when the scripted board supports them:

- `work_gate`: `real_work`, `reliability`, or `simplification`
- `real_work_score`: integer `1` to `5`

Use high scores only when the item directly improves the sacred workflow or protects a path users already touch.

## Enforcement

- Documentation must reference this doctrine for backlog intake, maintenance, and internal-agent recommendations.
- TODO items should declare a work gate when they are active or urgent.
- Sentinel must flag recurring doctrine violations as Observability findings.
- Governance and Validation may use Sentinel doctrine findings as release or review inputs, but Sentinel cannot auto-apply patches.
