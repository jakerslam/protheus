# Coding Workflow Evaluation Results

Purpose: track live coding-workflow reliability runs that are strong enough to guide promotion decisions.

## Result ledger

| Date | Gate | Harness | Attempts | Strict result | Evidence | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-05-14 | `coding_memory_live_level10_strict_20` | `coding_memory_live_level10_eval_execute` | 20 | 20 pass / 0 fail | `/tmp/level10_fresh_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level10-batch-98013-1778797455455` | Promote Level 10 as a required coding-workflow regression gate. |

## Level 10 gate meaning

Level 10 proves the workflow can run a long-form local coding continuation across two checkpoints with:

- local files treated as authoritative over memory
- roadmap and checkpoint receipts
- checkpoint memory writes
- durable operator workbench behavior
- SLO/policy escalation reporting
- snapshot export/import
- strict independent CLI semantic probing

## Level 11 target

Level 11 should start from the Level 10 foundation and test the next failure boundary:

- structured JSON errors for operator failure paths
- terminal status plus reopen lifecycle handling
- history-aware operator reports
- time-windowed SLO policy flags
- snapshot verification and snapshot diffing
- backward-compatible recovery round trips
