# Foundation Simplicity Budget Gate

`client/runtime/systems/ops/simplicity_budget_gate.ts` enforces complexity ceilings for the core `client/runtime/systems/` plane so architecture growth remains deliberate.

## Guarantees

- Total `client/runtime/systems/` file count and LOC budgets
- Per-organ file-count cap
- Primitive opcode-count cap
- Bespoke actuation-module cap
- New organ creation requires approved complexity-offset receipts
- Bespoke trend must stay non-increasing relative to captured baseline

## Policy

Policy file: `client/runtime/config/simplicity_budget_policy.json`

Baseline file: `client/runtime/config/simplicity_baseline.json`

Offset receipts: `state/ops/complexity_offsets.jsonl`

## Commands

```bash
# Capture baseline from current runtime shape
node client/runtime/systems/ops/simplicity_budget_gate.ts capture-baseline

# Evaluate budgets (strict fail-closed)
node client/runtime/systems/ops/simplicity_budget_gate.ts run --strict=1

# Read latest result
node client/runtime/systems/ops/simplicity_budget_gate.ts status
```
