# SRS Strict Execution

Generated: 2026-03-12T08:16:15.323Z

## Summary
- ok: true
- execute_now_before: 409
- queue_scanned: 1821
- queue_executed: 1821
- queue_failed: 0
- queue_skipped: 0
- queue_receipt_hash: 5f3094d98108f95aae5c355fc41e91ae94fde63ffd37362cdd583162ffb0ea74
- full_regression_fail: 0
- top200_regression_fail: 0
- execute_now_after: 409

## Steps
| Step | OK | Status | Command |
| --- | --- | --- | --- |
| srs_actionable_map:pre | true | 0 | `node scripts/ci/srs_actionable_map.mjs` |
| backlog_queue_executor:run_all_with_tests | true | 0 | `cargo run -q -p protheus-ops-core --bin protheus-ops -- backlog-queue-executor run --all=1 --with-tests=1` |
| srs_full_regression | true | 0 | `node scripts/ci/srs_full_regression.mjs` |
| srs_top200_regression | true | 0 | `node scripts/ci/srs_top200_regression.mjs` |
| srs_contract_runtime_evidence_test | true | 0 | `npm run -s test:ops:srs-contract-runtime-evidence` |
| verify | true | 0 | `./verify.sh` |
| srs_actionable_map:post | true | 0 | `node scripts/ci/srs_actionable_map.mjs` |
