# Capability Proof Burden Policy

## Purpose

Ensure every visible capability increase is backed by explicit, verifiable runtime truth.

## Scope

Applies to pull requests that add or expand user-visible capabilities, including any net-new or expanded:

- gateway features
- lane features
- shell-state features
- control-plane features

## Required Capability Proof Burden (PR-level)

For each new or expanded capability, PR authors must declare all of the following:

1. **Proof Artifact / Replay Fixture / Gate**
2. **Invariant**
3. **Failure Mode**
4. **Receipt Surface**
5. **Recovery Behavior**
6. **Verifiable Runtime Truth Increase**

## Fail-Closed Rule

Reject feature work that expands exterior capability without verifiable runtime truth increase.

A capability change is not admissible when:

- no proof artifact/replay fixture/gate is declared,
- invariants/failure behavior are undefined,
- receipt surface is missing,
- recovery behavior is unspecified,
- or claimed capability gain is not coupled to runtime-verifiable evidence.

## CI Guard Contract

CI guard must assert the active PR templates require the full capability proof burden fields and explicit fail-closed language.

- Guard script: `tests/tooling/scripts/ci/capability_proof_burden_guard.ts`
- Artifact: `core/local/artifacts/capability_proof_burden_guard_current.json`
- Report: `local/workspace/reports/CAPABILITY_PROOF_BURDEN_GUARD_CURRENT.md`

## Integration Notes

- This policy complements, and does not replace, existing DoD and SRS evidence requirements.
- Capability proof burden is required before a PR is considered releasable.
