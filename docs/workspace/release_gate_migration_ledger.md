# Release Gate Migration Ledger (V11-TODO-001)

Purpose: provide deterministic operator migration + rollback guidance for release-gated decisions so upgrades are auditable and reversible.

Version: `v1`
Last updated: `2026-04-19`

## Decision Ledger

| Decision | Contract Delta | Operator Migration Guidance | Rollback Guidance |
| --- | --- | --- | --- |
| `proof_pack_required_missing_zero` | Release blocks when proof pack reports missing required artifacts. | Run release candidate flow and confirm `required_missing: 0` before publish. Archive proof-pack summary with release notes. | Restore previous release policy file and rerun release gate if emergency rollback is required. |
| `proof_pack_category_completeness_gate` | Category completeness thresholds are enforced before release verdict. | Verify category summary entries are present for runtime/adapters/governance before tagging release. | Revert category threshold change and regenerate proof pack from prior config. |
| `layer2_lane_parity_guard_required` | Layer2 parity artifact is now release-blocking. | Ensure `layer2_lane_parity_guard_current.json` is generated and included in proof pack. | Revert parity guard requirement only with explicit incident waiver and add postmortem issue. |
| `layer2_receipt_replay_required` | Layer2 replay artifact is now release-blocking. | Run replay generation in RC flow and check artifact checksum in proof bundle. | Roll back to last known-good replay gate config and invalidate affected candidate tag. |
| `trusted_core_report_required` | Trusted-core report is now release-blocking. | Generate `runtime_trusted_core_report_current.json` and include in proof-pack checksums. | Revert trusted-core strictness gate only with documented waiver + follow-up date. |
| `dual_track_runtime_proof_gate` | Runtime proof tracks (`synthetic`, `empirical`) are enforced per-profile. | Confirm profile policy includes required track mode and nonzero empirical samples where configured. | Pin release to previous proof profile policy and rerun candidate verification. |
| `gateway_graduation_manifest_gate` | Adapter graduation uses manifest + scenario/hook completion checks. | Update adapter manifest with scenario coverage and verify all required chaos scenarios pass before release. | Revert manifest change to prior known-good state; mark affected adapters as non-graduated. |
| `dashboard_freshness_contract_gate` | Runtime block freshness fields are required (`source_sequence`, `age_seconds`, `stale`). | Verify dashboard runtime payloads include freshness metadata for all authority blocks before release. | Revert freshness enforcement policy and pin client to prior compatibility contract. |
| `node_critical_path_inventory_gate` | Node critical-path inventory is tracked as release evidence. | Update inventory artifact and confirm no untracked critical path regressions exist. | Restore previous inventory and open migration blocker before retry. |
| `windows_installer_fallback_contract` | Windows installer preflight + fallback reasons are deterministic and surfaced. | Verify release assets and fallback diagnostics are present in installer smoke report. | Revert installer contract changes and restore last known install script from prior release branch. |

## Release Candidate Checklist Linkage

1. Generate proof pack.
2. Validate all gate artifacts above are present.
3. Attach this ledger in release support bundle.
4. Publish release only if all ledger decisions are in `pass`.

## Operator Note

If any decision above is waived for emergency release, record:

- waiver reason
- approving owner
- affected release tag
- expiry date for waiver removal
