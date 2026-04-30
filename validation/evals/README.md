# Validation evals

Owner: Assurance / Validation
Status: physical migration in progress

This subdomain owns controlled eval definitions, scoring rubrics, reviewer labels, gold datasets, issue-candidate policies, and eval report destinations.

## Canonical assets

- `config/eval_quality_thresholds.json` defines eval quality thresholds, class-level precision/recall/FPR targets, actionability floors, calibration budgets, and regression guard limits.
- `policies/eval_issue_candidate_dedupe_policy.json` defines recurring-signature dedupe thresholds and proposal-only issue-candidate policy for eval outputs.
- `policies/eval_authority_calibration_policy.json` defines when eval observations can be authoritative and what longitudinal reviewer evidence is required before any closed-loop autonomy promotion.
- `policies/eval_feedback_lifecycle_policy.json` defines retention, cleanup, and protection tiers for eval and Kernel Sentinel feedback evidence.
- `policies/eval_issue_filing_policy.json` defines severity, confidence, persistence, and human-approval requirements for eval-generated issue filing.
- `policies/live_eval_policy.json` defines continuous live-eval sample sources, drift thresholds, and mitigation actions.
- `contracts/eval_issue_patch_links.json` tracks issue-to-patch closure links used by eval issue-resolution guards.
- `contracts/eval_issue_taxonomy.json` defines the canonical eval issue classes and severity scale used by dataset and issue-quality guards.
- `fixtures/eval_gold_dataset_v1.jsonl` is the controlled gold dataset for eval issue draft and quality-metrics checks.
- `fixtures/eval_learning_loop_review_labels.jsonl` is the controlled reviewer-label seed for the eval learning-loop review path.
- `fixtures/eval_*_cases.json`, `fixtures/eval_*_traces.json`, `fixtures/eval_*_telemetry.json`, and `fixtures/synthetic_user_chat_harness_cases.json` contain controlled eval regression, red-team, trajectory, synthetic-user, and learning-loop fixtures used by Orchestration eval runtimes.

## Compatibility rule

Runtime commands and tests may keep compatibility wrappers while callers migrate, but canonical eval definitions should live in this subdomain. New eval rubrics, datasets, labels, and scoring policies should not be added under `surface/orchestration/**` or `tests/tooling/config/**` unless explicitly marked as harness-only compatibility debt.

Current compatibility mirrors are tracked in `compatibility_mirrors.json` and should be burned down by the physical-domain placement guard work.

## Migrated plane eval contracts

- `contracts/eval_loop_contract_v1.json` is the canonical eval-loop contract formerly kept in the generic plane contract tree.
- `srs/` contains migrated eval SRS contract records.
- `fixtures/eval_adversarial_matrix.json` defines adversarial eval cases used by the eval adversarial guard.
- `fixtures/eval_gold_dataset_seed.jsonl` defines the seeded gold dataset rows used by the eval gold dataset schema guard.
- `schemas/eval_gold_dataset.schema.json` defines the controlled gold-dataset row schema used by eval dataset validation.
