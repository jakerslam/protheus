# Backlog Implementation Review

`systems/ops/backlog_implementation_review.ts` audits backlog rows and marks each as reviewed with deterministic evidence.

It verifies:
- implementation anchors exist for `done` rows
- wiring evidence exists (runtime + config/test/docs evidence)
- rows are not wrapper-only (`.js` launcher without substantive implementation)

## Commands

```bash
node systems/ops/backlog_implementation_review.js run
node systems/ops/backlog_implementation_review.js run --strict=1
node systems/ops/backlog_implementation_review.js status
```

## Policy

Policy file: `config/backlog_implementation_review_policy.json`

Outputs:
- review registry: `config/backlog_review_registry.json`
- reviewed view: `docs/backlog_views/reviewed.md`
- receipts: `state/ops/backlog_implementation_review/latest.json`, `state/ops/backlog_implementation_review/history.jsonl`
