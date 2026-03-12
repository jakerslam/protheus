# SRS Strict Execution

Generated: 2026-03-12T07:42:35.116Z

## Summary
- ok: false
- execute_now_before: null
- queue_scanned: null
- queue_executed: null
- queue_failed: null
- queue_skipped: null
- queue_receipt_hash: null
- full_regression_fail: null
- top200_regression_fail: null
- execute_now_after: null

## Steps
| Step | OK | Status | Command |
| --- | --- | --- | --- |
| srs_actionable_map:pre | true | 0 | `node scripts/ci/srs_actionable_map.mjs` |
| backlog_queue_executor:run_all_with_tests | true | 0 | `cargo run -q -p protheus-ops-core --bin protheus-ops -- backlog-queue-executor run --all=1 --with-tests=1` |
