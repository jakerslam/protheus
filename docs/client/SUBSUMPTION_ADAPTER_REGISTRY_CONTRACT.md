# Subsumption Adapter Registry Contract (RM-102)

Date: 2026-02-26  
Scope: `client/runtime/systems/eye/subsumption_registry.ts`

## Purpose

Provide governed provider/vassal adapter contracts with:
- trust-scored route evaluation
- per-provider and global token budgets
- rollback-safe disable/enable switch path
- auditable receipts for all mutations and evaluations

## Commands

```bash
node client/runtime/systems/eye/subsumption_registry.ts register --provider=<id> [--adapter=<id>] [--trust=0..1] [--daily-tokens=N] [--enabled=1|0]
node client/runtime/systems/eye/subsumption_registry.ts evaluate --provider=<id> [--estimated-tokens=N] [--risk=low|medium|high|critical]
node client/runtime/systems/eye/subsumption_registry.ts disable --provider=<id> --approval-note=<note> [--reason=<text>]
node client/runtime/systems/eye/subsumption_registry.ts enable --provider=<id> --approval-note=<note>
node client/runtime/systems/eye/subsumption_registry.ts status
```

## Decision Model

`evaluate` returns `allow | escalate | deny` using:
- provider existence + enable state
- trust thresholds:
  - `min_trust_allow`
  - `min_trust_escalate`
- risk gate (`high|critical` => deny)
- budgets:
  - provider `daily_tokens`
  - `global_daily_tokens`

## Policy / State / Receipts

- Policy: `client/runtime/config/subsumption_adapter_policy.json`
- State: `state/eye/subsumption_registry_state.json`
- Audit: `state/eye/audit/subsumption_registry.jsonl`
- Latest: `state/eye/subsumption_latest.json`

Every command appends auditable rows with normalized provider envelope and decision metadata.

## Tests

- `tests/client-memory-tools/subsumption_registry.test.js`

