## Summary

- What changed
- Why it changed

## Layer Ownership + Proof Gate Declaration (required)

- Which layer owns this, and which proof/gate covers it?
- Primary owner layer:
- Supporting layers touched:
- Primary proof artifact(s) / gate(s):

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

## Runtime Closure Feature Alignment (required for major surface features)

| Feature Surface | Scope (`major`/`minor`) | Runtime Closure Bucket | Validation Artifact / Gate |
| --- | --- | --- | --- |
|  |  |  |  |

- [ ] If any feature scope is `major`, each major feature maps to a runtime-closure bucket and directly validates it with a linked proof artifact, replay fixture, or release gate.

## Evidence

- Linked receipts/artifacts:

## Capability Proof Burden (required for new or expanded capabilities)

_Complete one row per new or expanded gateway/lane/shell-state/control-plane feature._

| Capability | Proof Artifact / Replay Fixture / Gate | Invariant | Failure Mode | Receipt Surface | Recovery Behavior | Verifiable Runtime Truth Increase |
| --- | --- | --- | --- | --- | --- | --- |
|  |  |  |  |  |  |  |

- [ ] No exterior capability expansion without verifiable runtime truth increase.
- [ ] Every visible capability change links to at least one proof artifact, replay fixture, or release gate.

## Capability Ownership + Proof Coverage (required for each net-new capability)

_Add one row per net-new capability introduced by this PR. Use canonical owner layer tokens: `kernel`, `control_plane`, `shell`, `gateway`, `apps`._

| Capability | Owner Layer | Proof / Gate Coverage |
| --- | --- | --- |
|  |  |  |
