# Neural Dormant Seed

`V3-023` keeps neural-interface work in a research-only locked lane.

## Commands

```bash
node client/runtime/systems/symbiosis/neural_dormant_seed_bridge.ts status --profile=prod
node client/runtime/systems/symbiosis/neural_dormant_seed_bridge.ts check --strict=1 --profile=prod
node client/runtime/systems/symbiosis/neural_dormant_seed_bridge.ts request-sim --purpose="evaluate consent signal contract"
node client/runtime/systems/symbiosis/neural_dormant_seed_bridge.ts request-live --purpose="prototype" --approval-note="manual"
```

`request-live` is expected to fail while policy is locked or profile is blocked.

## Policy

- `client/runtime/config/neural_dormant_seed_policy.json`
- Research artifacts:
  - `research/neural_dormant_seed/README.md`
  - `research/neural_dormant_seed/governance_checklist.md`
