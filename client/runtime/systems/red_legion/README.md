# Red Legion

Operational namespace for Red Legion roster and mission coordination.

## Commands

- `node client/runtime/systems/red_legion/command_center.ts enlist --operator-id=<id> [--alias=<name>] [--rank=recruit]`
- `node client/runtime/systems/red_legion/command_center.ts promote --operator-id=<id> --rank=<rank>`
- `node client/runtime/systems/red_legion/command_center.ts mission --operator-id=<id> --objective=<text> [--risk-tier=1..4]`
- `node client/runtime/systems/red_legion/command_center.ts status [--operator-id=<id>]`
