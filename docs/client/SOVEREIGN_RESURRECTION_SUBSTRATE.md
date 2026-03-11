# Sovereign Resurrection Substrate

`V3-RACE-037` composes cold archival, quantum-attestation checks, and resurrection drills into one continuity lane.

Entrypoint: `client/runtime/systems/continuity/sovereign_resurrection_substrate.ts`

## Commands

```bash
node client/runtime/systems/continuity/sovereign_resurrection_substrate.ts package --apply=1
node client/runtime/systems/continuity/sovereign_resurrection_substrate.ts drill --apply=1 --target-host=drill_host
node client/runtime/systems/continuity/sovereign_resurrection_substrate.ts status
```

Outputs include continuity hash attestations, bundle/verify/restore-preview receipts, and drill history.
