# Execution Sandbox Envelope

`V3-024` enforces policy-selected sandbox profiles for workflow and actuation execution.

## Commands

```bash
node client/runtime/systems/security/execution_sandbox_envelope.ts status
node client/runtime/systems/security/execution_sandbox_envelope.ts evaluate-workflow --step-id=step_1 --step-type=command --command="node script.js"
node client/runtime/systems/security/execution_sandbox_envelope.ts evaluate-actuation --kind=browser_automation --context='{"risk_class":"shell"}'
```

## Behavior

- Deny-by-default host filesystem and network access.
- Explicit capability manifests by sandbox profile.
- Escape-attempt token detection with audited deny events.
- High-risk actuation classes require explicit sandbox approval context.
