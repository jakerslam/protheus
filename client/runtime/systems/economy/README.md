# Compute-Tithe Flywheel (V3-RACE-022)

This lane implements donation -> validation -> tithe-discount application with receipted event flow.

## Commands

```bash
node client/runtime/systems/economy/public_donation_api.ts register --donor_id=alice
node client/runtime/systems/economy/public_donation_api.ts donate --donor_id=alice --gpu_hours=24 --proof_ref=tx123
node client/runtime/systems/economy/public_donation_api.ts status --donor_id=alice
node client/runtime/systems/economy/tithe_engine.ts status --donor_id=alice
node client/runtime/systems/economy/flywheel_acceptance_harness.ts --donor_id=sim --gpu_hours=240
node platform/api/donate_gpu.js donate --donor_id=alice --gpu_hours=24 --proof_ref=tx123
```

## Outputs

- `state/economy/contributions.json`
- `state/economy/donor_state.json`
- `state/economy/tithe_ledger.jsonl`
- `state/economy/receipts.jsonl`
- `state/blockchain/tithe_bridge_receipts.jsonl`
- integration hints under guard/fractal/routing/model/risk + soul patron marker lane
