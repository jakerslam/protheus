# Non-Shell ROI Wave - 2026-05-10

Scope: agent usefulness and anti-entropy work outside Shell and orchestration implementation paths.

## Completed in this wave

- Added unified trace identity fields to the Kernel Sentinel fresh-evidence guard.
- Added a bounded Sentinel full-run timeout report and guard.
- Added a Sentinel dream/full-run stage-split policy, report, and guard.
- Added a live Gateway status replay that classifies read-only status failures instead of treating them as opaque startup trouble.
- Patched the authoritative Rust dispatch-security source so `core://daemon-control status` is allowed as degraded read-only diagnostics when the embedded checker is unavailable.
- Rechecked safe-commit, command-runner-first, CI gate reduction, Sentinel evidence freshness, Sentinel timeout, Sentinel stage split, and Gateway live replay guards.

## Current live findings

- Sentinel full dream/self-study currently exceeds the 120s bounded run budget and emits a compact timeout diagnostic.
- Gateway read-only status currently fails before useful diagnostics because the dispatch path reports `embedded_security_checker_not_linked_use_cargo` with `cargo_fallback_disabled`.
- Source-level Cargo verification now passes for `infringctl gateway status`; the remaining npm replay failure indicates the installed/local wrapper binary is stale relative to source.
- The command runner is now useful by default: normal listing hides compatibility aliases unless `--include-compat=1` is requested.
- The CI required-gate reduction plan is ready: current required count is 48, target maximum is 18, and 38 checks are recommended for demotion.

## Next highest-ROI items

- Implement the Sentinel staged runner so dream/full self-study can resume across `evidence_collect`, `freshness_filter`, `root_cause_cluster`, `report_synthesis`, and `self_study`.
- Fix Gateway status dispatch so read-only status can run when the embedded security checker is not linked, without requiring restart/start recovery.
- Refresh or rebuild the installed CLI wrapper binary so `npm run gateway:status` exercises the patched source behavior.
- Continue converting the CI required-gate plan into actual required/advisory tiers.
- Add artifact retention budgets for observability and validation reports so anti-entropy data does not become entropy.
- Add focused Rust crate evidence for non-Shell, non-orchestration core crates.
