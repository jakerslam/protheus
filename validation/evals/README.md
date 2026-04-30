# Validation evals

Owner: Assurance / Validation
Status: physical migration in progress

This subdomain owns controlled eval definitions, scoring rubrics, reviewer labels, gold datasets, issue-candidate policies, and eval report destinations.

## Canonical assets

- `config/eval_quality_thresholds.json` defines eval quality thresholds, class-level precision/recall/FPR targets, actionability floors, calibration budgets, and regression guard limits.
- `policies/eval_issue_candidate_dedupe_policy.json` defines recurring-signature dedupe thresholds and proposal-only issue-candidate policy for eval outputs.
- `fixtures/eval_gold_dataset_v1.jsonl` is the controlled gold dataset for eval issue draft and quality-metrics checks.
- `fixtures/eval_learning_loop_review_labels.jsonl` is the controlled reviewer-label seed for the eval learning-loop review path.

## Compatibility rule

Runtime commands and tests may keep compatibility wrappers while callers migrate, but canonical eval definitions should live in this subdomain. New eval rubrics, datasets, labels, and scoring policies should not be added under `surface/orchestration/**` or `tests/tooling/config/**` unless explicitly marked as harness-only compatibility debt.

Current compatibility mirrors are tracked in `compatibility_mirrors.json` and should be burned down by the physical-domain placement guard work.
