# Economy Lane

`V3-RACE-022` adds a governed compute-tithe flywheel where verified donated GPU-hours reduce effective tithe and publish receipted events.

## Kernel Invariants

- Risk-tier defaults to `<=2`.
- Tier `3+` apply paths require second-gate approval.
- Every mutation emits ledger + receipt artifacts and event-stream publish evidence.
- Soul continuity marks GPU patrons in `state/soul/gpu_patrons.json`.

## Runtime Lanes

- `client/runtime/systems/economy/tithe_engine.ts`
- `client/runtime/systems/economy/gpu_contribution_tracker.ts`
- `client/runtime/systems/economy/contribution_oracle.ts`
- `client/runtime/systems/economy/tithe_ledger.ts`
- `client/runtime/systems/economy/smart_contract_bridge.ts`
- `client/runtime/systems/economy/public_donation_api.ts`
- `client/runtime/systems/economy/flywheel_acceptance_harness.ts`
- `platform/api/donate_gpu.ts` (open-platform compatibility API surface)
- `client/runtime/systems/economy/protheus_token_engine.ts` (`V3-RACE-130`)
- `client/runtime/systems/economy/global_directive_fund.ts` (`V3-RACE-130`)
- `client/runtime/systems/economy/peer_lending_market.ts` (`V3-RACE-133`)

## Data Scope Boundaries

- User-specific economy preferences/agreements:
  - `client/memory/economy/**`
  - `client/cognition/adaptive/economy/**`
- Permanent economy runtime/policy:
  - `client/runtime/systems/economy/**`
  - `client/runtime/config/*economy*` and related policy contracts
- Boundary enforcement:
  - `client/runtime/systems/ops/data_scope_boundary_check.ts`
  - `docs/client/DATA_SCOPE_BOUNDARIES.md`

## Collective Intelligence Economy Contract (`V3-RACE-160`)

- Incentive and access-tier runtime lanes:
  - `client/runtime/systems/economy/training_contributor_incentive_engine.ts`
  - `client/runtime/systems/economy/model_access_tier_governance.ts`
- Cross-lane integrity and scope check:
  - `client/runtime/systems/ops/collective_intelligence_contract_check.ts`
- Companion docs:
  - `docs/client/INTELLIGENCE.md`

## Quick Commands

```bash
node client/runtime/systems/economy/public_donation_api.ts register --donor_id=alice
node client/runtime/systems/economy/public_donation_api.ts donate --donor_id=alice --gpu_hours=24 --proof_ref=tx_1
node client/runtime/systems/economy/public_donation_api.ts status --donor_id=alice
node client/runtime/systems/economy/donor_mining_dashboard.ts dashboard
protheusctl mine dashboard --human=1
node client/runtime/systems/economy/flywheel_acceptance_harness.ts --donor_id=sim --gpu_hours=240
node platform/api/donate_gpu.js donate --donor_id=alice --gpu_hours=24 --proof_ref=tx_2
```
