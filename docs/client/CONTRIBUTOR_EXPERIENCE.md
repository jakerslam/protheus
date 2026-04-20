# Contributor Experience

## Goal

Make contribution predictable, testable, and safe for a fast-moving autonomy codebase.

## Local setup

```bash
npm install
npm run typecheck:systems
node client/runtime/systems/spine/contract_check_bridge.ts
```

## Workflow layer contribution rules

- Keep strategy policy in `client/runtime/config/strategies/`.
- Keep workflow definitions in `client/runtime/config/workflows/` + `state/client/cognition/adaptive/workflows/`.
- Do not mix workflow DAG logic into core strategy ranking code.

## Minimum checks before PR

```bash
node tests/client-memory-tools/strategy_principles.test.ts
node tests/client-memory-tools/workflow_controller.test.ts
node tests/client-memory-tools/collective_shadow.test.ts
node tests/client-memory-tools/observer_mirror.test.ts
node client/runtime/systems/ops/public_benchmark_pack.ts run
node client/runtime/systems/ops/deployment_packaging.ts run --profile=prod --strict=1
node client/runtime/systems/ops/compliance_posture.ts run --days=30 --profile=prod --strict=0
```

## Evidence expectations

- Include output JSON path(s) from benchmark and controller runs.
- Include policy/config changes with rationale.
- Include regression/behavior tests for new lanes.
