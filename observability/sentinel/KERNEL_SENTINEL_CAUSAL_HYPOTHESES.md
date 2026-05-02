# Kernel Sentinel Causal Hypotheses

Kernel Sentinel root-cause output must be a testable causal argument, not a clever summary.

## Purpose

Sentinel uses causal hypotheses to explain why a failure probably happened, what would prove the explanation wrong, and whether the finding is ready for TODO or issue promotion.

## Required Ladder

Every promoted hypothesis must include:

- `symptom`: what visibly failed.
- `immediate_mechanism`: the runtime mechanism that produced the symptom.
- `violated_invariant`: the Kernel, Gateway, Observability, or boundary law contradicted by evidence.
- `likely_root_cause`: the most likely cause that explains the evidence.
- `systemic_or_process_cause`: the deeper architecture/process cause that let the root cause exist.

## Required Evidence Shape

Every promoted hypothesis must carry:

- `support_evidence`: concrete evidence refs or compact evidence summaries.
- `counter_evidence`: facts that narrow, contradict, or scope the hypothesis.
- `missing_evidence`: evidence still needed before authority should increase.
- `confidence_percent`: confidence from evidence support, severity, and missing evidence.
- `causal_power_score`: how many symptoms/layers the hypothesis can explain.
- `falsification_probe`: a probe with expected results if the hypothesis is true or false.

## Promotion Rule

Sentinel may draft findings from hypotheses, but high-priority TODO or GitHub issue promotion should require:

- Confidence at or above the promotion threshold.
- Causal power high enough to explain more than one local symptom when symptoms are clustered.
- A falsification probe.
- Current evidence or explicit freshness metadata.
- Human review before automatic mutation or filing.

## Operator Use

Prefer the hypothesis with the strongest causal power, not the most visible symptom. A UI symptom, a gateway symptom, and a watchdog symptom may all point to one deeper cause such as fragmented runtime truth, authority-shaped residue, or invalid installed runtime identity.

## Calibration Loop

Sentinel causal hypotheses are now calibrated over time instead of treated as one-off prose.

- `causal_hypothesis_ledger_current.jsonl` records the current unresolved hypotheses from each run.
- `causal_hypothesis_outcomes.jsonl` is the durable human/Codex outcome ledger for `confirmed`, `partially_confirmed`, `contradicted`, and `unresolved` hypotheses.
- `causal_fix_results.jsonl` lets a later fix validate or contradict a hypothesis/pattern.
- `causal_pattern_scores_current.json` summarizes pattern trust from historical outcomes and fix results.
- `causal_calibration.final_report_summary` keeps the operator-facing report compact while raw history remains in ledgers.

Promotion is still draft-only. Sentinel may say a hypothesis is promotion-ready, but it must not mutate TODOs, file GitHub issues, or apply patches without review.
