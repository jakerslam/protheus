# Personal Infring Quickstart

## One-command install

```bash
node client/runtime/systems/security/operator_terms_ack.ts accept --operator-id=<id> --approval-note="initial_acceptance"
node client/runtime/systems/ops/personal_infring_installer.ts install
```

This writes:
- `state/ops/personal_infring/profile.json`
- `state/ops/personal_infring/install_manifest.json`

## Verify

```bash
node client/runtime/systems/ops/personal_infring_installer.ts status
```

## Recommended startup

```bash
node client/runtime/systems/spine/spine.ts daily
```

Start in `score_only` execution mode until readiness/guard checks are healthy.

## Legal Terms

Before contributing or deploying commercially, review:

- `LICENSE`
- `SECURITY.md`
- `docs/workspace/CONTRIBUTING.md`
- `docs/client/legal/archive/` (historical terms retained for audit context)
