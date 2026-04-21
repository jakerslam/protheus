# Release Lane Reviewer Redundancy Policy

## Purpose

Reduce bus-factor risk for release-critical lanes by requiring reviewer redundancy.

## Scope

Release-critical lanes include:

1. Release gating scripts under `tests/tooling/scripts/ci/release_*`
2. Runtime proof gates (`runtime_proof_*`, `layer2_*`, adapter chaos gate)
3. Release/policy docs under `docs/workspace/policy/**`

## Policy

1. Each scoped path must have at least two code owners.
2. At least one owner must be an operator for release orchestration.
3. Single-owner release-critical path changes require explicit exception note in the PR/release log.

## Enforcement

1. `CODEOWNERS` must keep redundant owners for scoped release-critical paths.
2. Release checklist review must include reviewer-redundancy confirmation.
3. CI guard command: `npm run -s ops:release-reviewer-redundancy:guard`.
4. Guard artifacts:
   - `core/local/artifacts/release_lane_reviewer_redundancy_guard_current.json`
   - `local/workspace/reports/RELEASE_LANE_REVIEWER_REDUNDANCY_GUARD_CURRENT.md`
