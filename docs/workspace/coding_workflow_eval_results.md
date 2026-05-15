# Coding Workflow Evaluation Results

Purpose: track live coding-workflow reliability runs that are strong enough to guide promotion decisions.

## Result ledger

| Date | Gate | Harness | Attempts | Strict result | Evidence | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-05-14 | `coding_memory_live_level10_strict_20` | `coding_memory_live_level10_eval_execute` | 20 | 20 pass / 0 fail | `/tmp/level10_fresh_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level10-batch-98013-1778797455455` | Promote Level 10 as a required coding-workflow regression gate. |
| 2026-05-14 | `coding_memory_live_level11_canary` | `coding_memory_live_level11_eval_execute` | 1 | 1 pass / 0 fail | `/tmp/level11_canary_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-10743-1778805016957` | Level 11 shape is coherent enough for small-batch testing. |
| 2026-05-14 | `coding_memory_live_level11_strict_5` | `coding_memory_live_level11_eval_execute` | 5 | 5 pass / 0 fail | `/tmp/level11_batch5_judge_rerun.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-19873-1778805644559` | Level 11 is ready for a 20-attempt reliability run, but not yet promoted. |
| 2026-05-14 | `coding_memory_live_level11_strict_20` | `coding_memory_live_level11_eval_execute` | 20 | 19 pass / 1 fail | `/tmp/level11_full20_judge_after_patch.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-37450-1778806499104` | Meets the 19/20 live reliability target after evaluator scoring repair; remaining real failure exposed deterministic SLO timestamp ambiguity. |

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
