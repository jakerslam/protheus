# Kernel Sentinel Release Bridge Repair Lane

Owner: Kernel Sentinel with Observability evidence support.

This lane turns release-evidence and receipt-integrity blockers into concrete repair work with current evidence, owner guesses, replay commands, and blocker classes.

## Release policy

- Human review is required before filing or promoting work.
- Auto-apply is forbidden.
- Release blocker authority requires current Sentinel evidence.
- Optimization suggestions must not be mixed into release-fail bridge blockers.

## Required bridge fields

- `release_evidence_artifact`
- `receipt_integrity_status`
- `source_artifact_freshness`
- `bridge_owner`
- `replay_command`
- `blocker_class`

## Acceptance criteria

- Release-evidence blockers identify the missing or stale artifact and cite a current-live-truth or recent-but-not-current freshness tier.
- Receipt-integrity blockers name the failed receipt surface, expected invariant, observed mismatch, and replay command.
- Sentinel final reports group release-evidence and receipt-integrity failures separately from optimization suggestions.
- A release-fail verdict cannot promote a bridge issue candidate without owner guess, blocker class, evidence refs, and concrete next action.

## Validation commands

```bash
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::release_gate_synthesis -- --nocapture
cargo test --manifest-path core/layer0/ops/Cargo.toml --lib kernel_sentinel::root_tests::strict_report_fails_on_open_critical_findings -- --exact --nocapture
npm run -s ops:ksent:release-bridge-repair:guard
```
