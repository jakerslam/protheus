# Recovery Truth Table (Production Contract)

This table defines what survives, what degrades, and what is blocked when a subsystem fails.

| Subsystem Failure | Durable Truth | In-Flight Work | Recovery Source | Allowed Mode During Failure |
| --- | --- | --- | --- | --- |
| Presentation Client | Preserved in Core | Resumable via Task Fabric | Core receipts + task state | Degraded UI only |
| Orchestration Surface | Preserved in Core | Transient orchestration context may be dropped | Core contracts + receipts | Rebuild orchestration state from Core |
| Core authority process | Unavailable for mutation | No canonization allowed | Restart Core and replay receipts | Read-only/degraded wrappers only |
| IPC bridge disconnect | Preserved | Request fails closed unless retry policy allows | Reconnect resident daemon bridge | No process fallback in production channel |
| Runtime transient context write failure | Preserved | Flow halts before planning | Retry ephemeral write | `orchestration_degraded` fail-closed response |
| Release upgrade incompatibility | Preserved if gate catches pre-deploy | Block rollout | Compatibility gate + migration policy | Upgrade denied |
| Rollback request | Preserved if rollback contract passes | Resume from compatible durable state | rollback lane + receipts | Controlled rollback only |
| Telemetry pipeline failure | Preserved | Operational visibility reduced | telemetry lane restart | Runtime continues with degraded observability |
| Installer post-check failure | Existing state preserved | New install not promoted as healthy | installer doctor + support bundle | install marked degraded; no silent success |

## Operator Commands

- Topology check: `npm run -s ops:transport:topology:gate`
- Release contract gate: `npm run -s ops:release-contract:gate`
- Reliability gauntlet: `npm run -s ops:reliability:gauntlet`
- IPC soak: `npm run -s ops:ipc-bridge:soak`
- Support bundle export: `npm run -s ops:support-bundle:export`
