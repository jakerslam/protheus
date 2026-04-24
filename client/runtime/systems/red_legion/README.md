# Red Legion

Shell-side namespace marker only. Red Legion command authority is core-owned.

## Commands

- `infring session register --session-id=<id> [--lineage-id=<id>] [--task=<text>]`
- `infring session resume <id>`
- `infring session send <id> --message=<text>`
- `infring session kill <id>`
- `infring session tail <id> [--lines=<n>]`
- `infring session inspect <id>`
- `infring-ops command-center-session status [--session-id=<id>]`

Do not add authoritative roster, mission, or session-control logic under `client/runtime/systems/red_legion/`.
