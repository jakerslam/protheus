# TODO (Blocker Queue)

Updated: 2026-03-10

Backlog implementation is paused until runtime validation is real (non-deferred).

## Priority 0 - Runtime blockers

1. `OPS-BLOCKER-001` Unblock local binary execution. `STATUS: BLOCKED`
- Exit criteria:
- `/tmp/hello_c_test` executes and exits `0` with expected stdout.
- `/tmp/hello_rust_test` executes and exits `0` with expected stdout.
- `./target/debug/protheus-ops status` returns a JSON receipt without timeout.
- Current evidence:
- `/tmp/hello_c_test` hangs (`ETIMEDOUT` / no stdout).
- `/tmp/hello_rust_test` hangs (`ETIMEDOUT` / no stdout).
- `spctl --assess --type execute /tmp/hello_c_test` => rejected.

2. `OPS-BLOCKER-002` Remove deferred host-stall fallback from validation path. `STATUS: COMPLETE`
- Exit criteria:
- No `ops_domain_deferred_host_stall` receipts during validation commands.
- Validation wrappers fail closed on host stall/timeouts.
- Completion notes:
- `client/runtime/lib/rust_lane_bridge.ts` default changed to `PROTHEUS_OPS_DEFER_ON_HOST_STALL=0`.
- `client/runtime/lib/legacy_retired_wrapper.js` default changed to `PROTHEUS_OPS_DEFER_ON_HOST_STALL=0`.
- Validation now returns hard failures (`spawnSync .../protheus-ops ETIMEDOUT`) instead of deferred receipts.

3. `OPS-BLOCKER-003` Re-run full regression with real runtime execution. `STATUS: PARTIAL (runtime blocked)`
- Exit criteria:
- `./verify.sh` completes without deferred-host-stall receipts.
- System test suite reports real pass/fail outcomes.
- Current evidence:
- Full snapshot artifact: `artifacts/blocker_regression_2026-03-10.json`.
- Non-runtime checks pass (`ops:srs:top200:regression`, `metrics:rust-share:gate`, `ops:layer-placement:check`).
- Runtime checks fail-closed on local binary timeout until blocker 001 is resolved.

4. `COREIZATION-GATE-001` Keep client authority surfaces wrapper-only. `STATUS: COMPLETE`
- Exit criteria:
- `node scripts/ci/coreization_wave1_static_audit.mjs` -> `pass: true`.
- `npm run -s ops:layer-placement:check` -> `violations_count: 0`.

5. `BACKLOG-RESUME` Resume ROI backlog execution only after blockers 1-4 pass. `STATUS: BLOCKED`
- Exit criteria:
- Blockers 1-4 marked complete in this file and checkpoint doc.

## Commands

- `node scripts/ci/coreization_wave1_static_audit.mjs --out artifacts/coreization_wave1_static_audit_2026-03-10.json`
- `npm run -s ops:layer-placement:check`
- `npm run -s metrics:rust-share:gate`
- `npm run -s ops:srs:top200:regression`
