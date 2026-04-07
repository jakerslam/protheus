# Compression Transfer Plane

`RM-126` adds deterministic state compression/expansion for phone `<->` desktop/cluster profile movement.

## Commands

```bash
node client/runtime/systems/hardware/compression_transfer_plane.ts compress
node client/runtime/systems/hardware/compression_transfer_plane.ts expand --bundle-id=<id> --apply=1
node client/runtime/systems/hardware/compression_transfer_plane.ts auto --target-profile=desktop --apply=1
node client/runtime/systems/hardware/compression_transfer_plane.ts status
```

## Guarantees

- Bundle includes policy-scoped state files with SHA256 attestation digest
- Expand verifies attestation before any restore write
- Receipts are replayable (`state/hardware/compression_transfer_plane/receipts.jsonl`)
- Auto mode chooses `compress`/`expand` from profile rank (`phone < desktop < cluster`)

## Policy

Policy file: `client/runtime/config/compression_transfer_plane_policy.json`

Primary knobs:
- `include_paths` (state files to move into dormant bundle)
- `strict_default`, `apply_default`
- `bundle_dir`, `latest_path`, `receipts_path`
