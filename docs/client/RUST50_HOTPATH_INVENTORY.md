# Rust50 Hotpath Inventory

Use the inventory tool to rank migration order by measured TypeScript line concentration.

## Commands

```bash
node client/runtime/systems/ops/rust_hotpath_inventory.ts run
node client/runtime/systems/ops/rust_hotpath_inventory.ts status
node client/runtime/systems/ops/top50_roi_sweep.ts --max=50
node client/runtime/systems/ops/top50_roi_sweep.ts --max=100
```

## Outputs

- Latest snapshot: `state/ops/rust_hotpath_inventory/latest.json`
- History ledger: `state/ops/rust_hotpath_inventory/history.jsonl`

Each run emits:

- `tracked_ts_lines` / `tracked_rs_lines` / `rust_percent`
- top directories by line volume
- top files by line volume
- milestone math (`additional_rs_lines_needed`) for target percentages

This keeps Rust migration sequencing anchored to measured impact, not ad-hoc prioritization.

## RUST60 Ranked Queue

Generated execution artifacts:

- Full ranked TS hotpaths (all files): `docs/client/generated/RUST60_TS_HOTPATHS_RANKED_FULL.csv`
- Full ranked TS hotpaths (markdown): `docs/client/generated/RUST60_TS_HOTPATHS_RANKED_FULL.md`
- 60% execution queue (rank 1-261): `docs/client/generated/RUST60_EXECUTION_QUEUE_261.json`
- 60% execution queue (markdown): `docs/client/generated/RUST60_EXECUTION_QUEUE_261.md`

Queue policy:

- Process lanes strictly in rank order.
- Commit and push each lane independently.
- Keep lane diffs isolated to minimize rollback scope.
- Exclude thin bridge/client wrapper surfaces (`createOpsLaneBridge`, conduit client, bridge/bootstrap glue) from the live queue.
- Treat the queue as an opportunity ranking even when repository-wide Rust share already exceeds the historical 60% threshold.
