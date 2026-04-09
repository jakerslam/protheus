# Organ State Encryption Plane

`V3-025` provides per-organ encryption for state, memory, and cryonics lanes with key-versioning, integrity MAC, rotation, and fail-closed decrypt denial.

## Commands

```bash
node client/runtime/systems/security/organ_state_encryption_plane.ts encrypt --organ=workflow --lane=state --source=state/example.json
node client/runtime/systems/security/organ_state_encryption_plane.ts decrypt --organ=workflow --cipher=state/example.json.enc.json --out=state/example.restored.json
node client/runtime/systems/security/organ_state_encryption_plane.ts rotate-key --organ=workflow --reason="scheduled_rotation"
node client/runtime/systems/security/organ_state_encryption_plane.ts verify --strict=1
node client/runtime/systems/security/organ_state_encryption_plane.ts status
```

## Guarantees

- Per-organ keyring with active key version and historical versions for decrypt continuity.
- `aes-256-gcm` confidentiality plus explicit `hmac-sha256` envelope integrity MAC.
- Rotation receipts and decrypt receipts are append-only.
- Unauthorized decrypt attempts fail closed and emit system health alerts.
