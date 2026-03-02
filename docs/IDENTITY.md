# Identity Lane

Identity contracts in this workspace are split into three scopes:

- User-specific state:
  - `memory/identity/` for owner-facing identity preferences/history.
  - `adaptive/identity/` for adaptive tuning metadata.
- Permanent runtime logic:
  - `systems/identity/` for deterministic engines and verifiers.
  - `config/` for policy contracts and limits.

## Active Identity Runtime Lanes

- `systems/identity/identity_organ.ts`
- `systems/identity/identity_integrity_oracle.ts`
- `systems/identity/visual_signature_engine.ts` (`V3-RACE-134`)
- `systems/contracts/soul_contracts.ts` (`V3-RACE-129`)

## Invariants

- Runtime identity decisions are receipted.
- Tier `3+` actions require explicit approval.
- Visual signature manifests are deterministic and hash-verifiable.
- User identity history stays in `memory/` and `adaptive/`, not in `systems/`.
