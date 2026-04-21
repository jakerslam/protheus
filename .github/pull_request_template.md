## Summary

- What changed
- Why it changed

## Validation

- [ ] `npm run test`
- [ ] `cargo test --manifest-path core/layer0/ops/Cargo.toml`
- [ ] `cargo clippy --manifest-path core/layer0/ops/Cargo.toml --all-targets -- -D warnings`
- [ ] `npm run -s formal:invariants:run`
- [ ] If this PR crosses ownership layers, I completed **Ownership Placement Rationale** and included placement-test evidence.

## Risk + Rollback

- Risk class: standard
- RFC link:
- ADR link:
- Rollback owner:
- Rollback plan:
- Approvers:
- Approval receipts:
- Rollback drill receipt:

## Ownership Placement Rationale (required when touching multiple ownership zones)

- Changed ownership zones:
- Policy reference: `docs/workspace/orchestration_ownership_policy.md`
- Placement test references:
  - `placement-test:core` —
  - `placement-test:control-plane` —
  - `placement-test:shell` —
  - `placement-test:gateway` —
  - `placement-test:apps` —

## Evidence

- Linked receipts/artifacts:

## Capability Proof Burden (required for new or expanded capabilities)

_Complete one row per new or expanded gateway/lane/shell-state/control-plane feature._

| Capability | Proof Artifact / Replay Fixture / Gate | Invariant | Failure Mode | Receipt Surface | Recovery Behavior | Verifiable Runtime Truth Increase |
| --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |

- [ ] No exterior capability expansion without verifiable runtime truth increase.
