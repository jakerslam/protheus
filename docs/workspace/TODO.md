# TODO (Priority + ROI + Dependency Ordered)

Updated: 2026-03-10 (SRS-synced refresh + execution tranche applied)

## Ordering policy
- Priority first (`P0` > `P1` > `P2` > `P3`)
- Then ROI (higher unblock value first)
- Then dependency chain (prerequisites before dependents)

## Backlog snapshot
- Source: `docs/workspace/SRS.md` + `client/runtime/config/backlog_registry.json`
- Latest actionable report: `artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`
- Counts: `queued=373`, `in_progress=1`, `blocked=42`, `done=2227`

## Ordered execution queue

1. `MAINT-001` `P0` `ROI=10/10` `DEP=none` Refresh TODO from live SRS/backlog state. `STATUS: COMPLETE`
- Exit criteria:
- TODO reflects current SRS statuses and dependency-aware ordering.
- Completion evidence:
- `docs/workspace/TODO.md`
- `artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`

2. `V6-SEC-008` `P0` `ROI=10/10` `DEP=V6-SEC-003` Continuous Fuzzing + Chaos Suite closure. `STATUS: COMPLETE`
- Exit criteria:
- Nightly workflow emits deterministic fuzz/chaos report artifacts.
- Triage policy exists and is linked in security policy.
- Completion evidence:
- `.github/workflows/nightly-fuzz-chaos.yml`
- `scripts/ci/nightly_fuzz_chaos_report.mjs`
- `docs/client/FUZZ_CHAOS_TRIAGE.md`
- `SECURITY.md`
- `artifacts/nightly_fuzz_chaos_report_latest.json`

3. `MAINT-002` `P0` `ROI=9/10` `DEP=001,002` Post-change gate/regression pass. `STATUS: COMPLETE`
- Exit criteria:
- Primitive wrapper contract gate passes.
- Coreization static audit passes.
- Rust-share gate remains above 60%.
- `verify.sh` passes.
- Completion evidence:
- `./target/debug/protheus-ops contract-check --rust-contract-check-ids=primitive_ts_wrapper_contract`
- `artifacts/coreization_wave1_static_audit_2026-03-10_todo_refresh.json`
- `npm run -s metrics:rust-share:gate` (`64.849%`)
- `./verify.sh`

4. `MAINT-003` `P1` `ROI=8/10` `DEP=003` Refresh actionable backlog artifact after tranche execution. `STATUS: COMPLETE`
- Exit criteria:
- New actionable artifact generated from current SRS/TODO.
- Completion evidence:
- `artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`

5. `V6-SEC-001` `P1` `ROI=9/10` `DEP=V6-F100-003` Audited Release + SBOM bundle (`v0.2.0`). `STATUS: IN_PROGRESS`
- Current state:
- Required scaffolding already exists:
  - `.github/workflows/release-security-artifacts.yml`
  - `docs/client/RELEASE_SECURITY_CHECKLIST.md`
  - `docs/client/releases/v0.2.0_migration_guide.md`
- Remaining closure condition:
- Human-authorized tagged release publication and artifact verification record.

6. `COREIZATION-NEXT-001` `P1` `ROI=9/10` `DEP=003` Deep authority migration (core-first) for remaining TS heavy surfaces. `STATUS: IN_PROGRESS`
- Scope:
- `client/runtime/lib/strategy_resolver.ts` -> `core/layer2/execution` authoritative path
- `client/runtime/lib/duality_seed.ts` -> `core/layer2/autonomy` authoritative path
- Exit criteria:
- TS files reduced to thin conduit wrappers only.
- Rust crate lanes carry source-of-truth behavior and pass parity tests.

7. `V6-SEC-004` `P2` `ROI=7/10` `DEP=V6-SEC-001,V6-SEC-003` Independent security audit publication. `STATUS: QUEUED`

8. `V6-SEC-005` `P2` `ROI=7/10` `DEP=V6-SEC-002,V6-SEC-004` Formal verification expansion package. `STATUS: QUEUED`

9. `V6-F100-025` `P2` `ROI=6/10` `DEP=human cadence` Weekly chaos evidence cadence contract. `STATUS: BLOCKED`
- Blocker:
- Requires sustained weekly operational cadence + human-owned evidence publication.

10. `V7-META-FOUNDATION` `P3` `ROI=8/10` `DEP=coreization-next` Metakernel foundation wave (`V7-META-001..015`). `STATUS: QUEUED`
- Notes:
- Keep queued until `COREIZATION-NEXT-001` is closed to avoid splitting authority lanes.

## Commands used in this tranche
- `node scripts/ci/nightly_fuzz_chaos_report.mjs`
- `./target/debug/protheus-ops contract-check --rust-contract-check-ids=primitive_ts_wrapper_contract`
- `node scripts/ci/coreization_wave1_static_audit.mjs --out artifacts/coreization_wave1_static_audit_2026-03-10_todo_refresh.json`
- `npm run -s metrics:rust-share:gate`
- `./verify.sh`
- `node scripts/ci/backlog_actionable_report.mjs --out artifacts/backlog_actionable_report_2026-03-10_todo_refresh.json`
