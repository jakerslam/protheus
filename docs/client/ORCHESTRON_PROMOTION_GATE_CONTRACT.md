# Orchestron Promotion Gate Contract

Status: active (`RM-013`)

Source implementation:
- `client/runtime/systems/workflow/workflow_controller.ts` (`promote` command)
- `client/runtime/config/workflow_policy.json` (`promotion_gate`)

## Required Promotion Gates

Before any `promotable_draft` is applied to active registry, promotion gate enforces:

1. Contract fields
- `id`, `name`, `trigger.proposal_type`
- `steps[]` present
- gate step present (`type: "gate"`)
- receipt step present (`type: "receipt"`)
- `metrics.score` present

2. Non-regression metrics (active promotion)
- `predicted_drift_delta <= max_predicted_drift_delta`
- `predicted_yield_delta >= min_predicted_yield_delta`
- `safety_score >= min_safety_score`
- `regression_risk <= max_regression_risk`
- snapshot red-team critical failures within policy cap

3. Approval receipt (active non-dry-run promotion)
- `--approver-id=<id>`
- `--approval-note="..."`

## Promotion Receipts

Each promote run appends a gate receipt to:
- `state/client/cognition/adaptive/workflows/promotion_receipts/<date>.jsonl`

Receipt includes:
- selected / eligible / blocked counts
- blocked-by-reason summary
- approval metadata presence
- snapshot red-team status
- applied / updated counts

## CLI Contract

```bash
node client/runtime/systems/workflow/workflow_controller.ts promote \
  --source=promotable \
  --status=active \
  --ignore-threshold=1 \
  --approver-id=<id> \
  --approval-note="..." \
  --policy=client/runtime/config/workflow_policy.json
```

## Verification

```bash
node tests/client-memory-tools/workflow_controller_promote.test.js
node tests/client-memory-tools/workflow_controller_identity_gate.test.js
node tests/client-memory-tools/workflow_controller_promotion_gate.test.js
node client/runtime/systems/spine/contract_check.ts
```
