# Rust50 Hotpath Inventory

Use the inventory tool to rank migration order by measured TypeScript line concentration.

## Commands

```bash
node systems/ops/rust_hotpath_inventory.js run
node systems/ops/rust_hotpath_inventory.js status
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
