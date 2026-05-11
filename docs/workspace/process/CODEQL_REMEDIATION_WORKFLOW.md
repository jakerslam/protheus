# CodeQL Remediation Workflow

Status: canonical real-work workflow.

This is the first official "alert to patch" loop under the Three Operating Laws.

## Goal

Turn a concrete CodeQL/security alert into a narrow fix, targeted validation, and traceable closure pattern.

## Workflow

1. Capture alert ID, rule ID, severity, file, line, and recommendation.
2. Locate the smallest owning source file and behavior.
3. Identify whether the alert is:
   - true positive
   - stale generated artifact
   - test-only false positive
   - policy/documentation gap
4. Patch narrowly.
5. Run targeted validation for the touched path when appropriate.
6. Commit with a conventional prefix.
7. Record the closure pattern if the same rule may recur.

## Required Output

Each remediation should report:

- alert ID and rule ID
- changed files
- behavior preserved or intentionally changed
- validation performed or intentionally deferred
- residual risk

## Three Operating Laws Mapping

- `usability`: security alerts are concrete external work with visible user trust impact.
- `reliability`: security fixes must preserve install, runtime, and release behavior.
- `simplicity`: recurring alert classes should become reusable fix patterns rather than bespoke churn.

## Sentinel Link

Sentinel may promote recurring CodeQL remediation gaps as reliability findings when they block release confidence or repeatedly produce unsafe/stale patterns.
