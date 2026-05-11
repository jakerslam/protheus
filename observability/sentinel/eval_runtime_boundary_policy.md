# Kernel Sentinel / Eval Runtime Boundary Policy

Kernel Sentinel and eval agents are complementary, but they must not grade the same failure as if they own the same scope.

## Sentinel owns runtime-system failures

Sentinel should focus on:

- Receipt drift, missing finalization, stale state, authority leaks, cadence failures, and persistence gaps.
- Kernel, Observability, Governance, Gateway boundary, and runtime topology health.
- Structural root-cause clustering across live evidence.
- Anti-entropy recommendations tied to the Three Operating Laws.

## Eval owns response-quality failures

Eval should focus on:

- Hallucinations, wrong tool choice, missing tool call, bad answer quality, refusal/response policy drift, and non-response behavior.
- Golden traces, controlled fixtures, benchmark suites, and scorecards.
- Model/workflow behavior under controlled checks.

## Handoff rule

Sentinel may cite response failures only when they are evidence of a runtime-system failure, such as missing finalization, broken trace propagation, stale gateway data, or non-fresh evidence. Otherwise it should route the issue to eval.

Eval may cite runtime failures only when they invalidate controlled scoring. Otherwise it should route the issue to Sentinel.

## Shared issue rule

A joint issue is allowed only when it contains:

- Sentinel-owned runtime evidence.
- Eval-owned controlled-check evidence.
- A clear owner split.
- A single root-cause hypothesis or an explicit multi-root declaration.
