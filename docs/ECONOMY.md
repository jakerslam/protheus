# Economy Lane

`V3-RACE-022` adds a governed compute-tithe flywheel where verified donated GPU-hours reduce effective tithe and publish receipted events.

## Core Invariants

- Risk-tier defaults to `<=2`.
- Tier `3+` apply paths require second-gate approval.
- Every mutation emits ledger + receipt artifacts and event-stream publish evidence.
- Soul continuity marks GPU patrons in `state/soul/gpu_patrons.json`.

## Runtime Lanes

- `systems/economy/tithe_engine.ts`
- `systems/economy/gpu_contribution_tracker.ts`
- `systems/economy/contribution_oracle.ts`
- `systems/economy/tithe_ledger.ts`
- `systems/economy/smart_contract_bridge.ts`
- `systems/economy/public_donation_api.ts`
- `systems/economy/flywheel_acceptance_harness.ts`
- `platform/api/donate_gpu.ts` (open-platform compatibility API surface)
- `systems/economy/protheus_token_engine.ts` (`V3-RACE-130`)
- `systems/economy/global_directive_fund.ts` (`V3-RACE-130`)
- `systems/economy/peer_lending_market.ts` (`V3-RACE-133`)

## Data Scope Boundaries

- User-specific economy preferences/agreements:
  - `memory/economy/**`
  - `adaptive/economy/**`
- Permanent economy runtime/policy:
  - `systems/economy/**`
  - `config/*economy*` and related policy contracts
- Boundary enforcement:
  - `systems/ops/data_scope_boundary_check.ts`
  - `docs/DATA_SCOPE_BOUNDARIES.md`

## Collective Intelligence Economy Contract (`V3-RACE-160`)

- Incentive and access-tier runtime lanes:
  - `systems/economy/training_contributor_incentive_engine.ts`
  - `systems/economy/model_access_tier_governance.ts`
- Cross-lane integrity and scope check:
  - `systems/ops/collective_intelligence_contract_check.ts`
- Companion docs:
  - `docs/INTELLIGENCE.md`

## Quick Commands

```bash
node systems/economy/public_donation_api.js register --donor_id=alice
node systems/economy/public_donation_api.js donate --donor_id=alice --gpu_hours=24 --proof_ref=tx_1
node systems/economy/public_donation_api.js status --donor_id=alice
node systems/economy/flywheel_acceptance_harness.js --donor_id=sim --gpu_hours=240
node platform/api/donate_gpu.js donate --donor_id=alice --gpu_hours=24 --proof_ref=tx_2
```
