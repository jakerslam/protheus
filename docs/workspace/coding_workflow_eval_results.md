# Coding Workflow Evaluation Results

Purpose: track live coding-workflow reliability runs that are strong enough to guide promotion decisions.

Timing policy: new leveled coding evals should report speed as well as correctness. Fresh seed reports include `seed_started_at_unix_ms`; strict judge reports include `timing.batch_elapsed_ms`, `timing.completion_span_ms`, `timing.average_attempt_elapsed_ms`, and per-attempt `attempts[].timing.elapsed_ms_since_batch_start`.

Execution-source policy: Codex `spawn_agent` runs validate workflow prompts, seed projects, and judges, but they do not prove Infring-native agent capability. Infring-native capability evidence must come from Infring agent creation/provider/tooling paths and Infring receipts.

## Result ledger

| Date | Gate | Harness | Attempts | Strict result | Evidence | Decision |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-05-14 | `coding_memory_live_level10_strict_20` | `coding_memory_live_level10_eval_execute` | 20 | 20 pass / 0 fail | `/tmp/level10_fresh_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level10-batch-98013-1778797455455` | Promote Level 10 as a required coding-workflow regression gate. |
| 2026-05-14 | `coding_memory_live_level11_canary` | `coding_memory_live_level11_eval_execute` | 1 | 1 pass / 0 fail | `/tmp/level11_canary_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-10743-1778805016957` | Level 11 shape is coherent enough for small-batch testing. |
| 2026-05-14 | `coding_memory_live_level11_strict_5` | `coding_memory_live_level11_eval_execute` | 5 | 5 pass / 0 fail | `/tmp/level11_batch5_judge_rerun.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-19873-1778805644559` | Level 11 is ready for a 20-attempt reliability run, but not yet promoted. |
| 2026-05-14 | `coding_memory_live_level11_strict_20` | `coding_memory_live_level11_eval_execute` | 20 | 19 pass / 1 fail | `/tmp/level11_full20_judge_after_patch.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-37450-1778806499104` | Meets the 19/20 live reliability target after evaluator scoring repair; remaining real failure exposed deterministic SLO timestamp ambiguity. |
| 2026-05-15 | `coding_memory_live_level11_strict_20_fresh_after_slo_guardrail` | `coding_memory_live_level11_eval_execute` | 20 | 19 pass / 1 fail | `/tmp/level11_fresh20_after_guardrail_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-33414-1778811593115` | Fresh run confirms the SLO timestamp guardrail repaired the prior failure class; remaining real failure exposed same-second operator event replay ordering. |
| 2026-05-15 | `coding_memory_live_level11_strict_20_after_event_ordering_guardrail` | `coding_memory_live_level11_eval_execute` | 20 | 20 pass / 0 fail | `/tmp/level11_rerun_after_event_ordering_judge.json`; batch root `/var/folders/f9/mhsd3dwj78l8418t9vnclbn80000gn/T/coding-memory-live-level11-batch-9911-1778814884387`; timing `batch_elapsed_ms=2323350`, `completion_span_ms=1843268`, `average_attempt_elapsed_ms=1282085`; worker runtime `Codex spawn_agent` | Promote Level 11 prompt/judge shape as workflow evidence only; not proof of Infring-native agent capability. |
| 2026-05-15 | `coding_memory_infring_native_agent_execution_smoke` | `cargo run -p xtask -- infring-agent-run` | 1 | 0 pass / 1 fail | `/tmp/infring_agent_run_ollama_smoke.json`; failure `provider_not_registered:ollama` | Native Infring runtime lane exists, but the intended Ollama/Kimi provider is not registered in `infring_agent_surface`; coding evals must not rely on Codex workers for capability claims. |

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

## Level 12 target

Level 12 starts from an existing multi-module codebase with baseline tests and checks whether the coding workflow can evolve it safely instead of producing a detached greenfield slice.

- preserve existing `item-create`, `item-complete`, and `queue-summary` contracts
- add idempotent `bulk-import` behavior over external CSV data
- add hold-aware deterministic `sla-report` behavior using explicit `--as-of`
- add rollback-safe `state-export`, `state-import`, and `state-diff`
- require roadmap, checkpoint receipts, memory writes, timing, and a strict post-worker semantic CLI probe

## Native Coding Useful-Work Eval v1

This is the official coding wedge scoreboard for Infring-native agent capability. It is stricter than receipt-only harnesses:

- baseline validation without source/test mutation is failure
- new regression tests must be exercised
- expected public symbols must exist
- an independent semantic probe must pass
- native receipt evidence must include mutation receipts
- timing must report batch speed and time to first mutation

Harness: `native_coding_useful_work_eval_execute`

Promotion target: `19/20` Infring-native passes. Codex subagent runs may validate prompt and judge shape only; they do not prove native Infring coding capability.
