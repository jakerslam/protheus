# OUROBOROS Adaptations for Protheus

Updated: 2026-02-25

## P0 (Build Immediately)

1. Pulsed Background Suggestions
- Status: implemented
- Where:
  - `systems/autonomy/suggestion_lane.ts`
  - `systems/spine/spine.ts` (`spine_suggestion_lane`)
- Notes: capped daily merge lane for pulsed suggestions; proposal-only output.

2. Proposal Decomposition Engine
- Status: implemented (operational + dependency summary)
- Where:
  - `systems/security/directive_hierarchy_controller.ts`
  - `systems/autonomy/autonomy_controller.ts` (`proposal_dependencies`)
- Notes: proposal decomposition already active; dependency metadata now attached to autonomy receipts.

3. Constitution Visualization (IDE)
- Status: implemented
- Where:
  - `agent-holo-viz/server/system_visualizer_server.js`
  - `agent-holo-viz/client/app.js`
- Notes: alignment scoring and T1/T2 visibility now appear in visualizer summary panes.

## P1 (Build This Week)

4. Multi-Signal Proposal Review
- Status: implemented
- Where:
  - `systems/autonomy/autonomy_controller.ts` (`preexec_verdict`)
  - `systems/autonomy/canary_scheduler.ts`
- Notes: readiness verdict now includes strategy, governance, dopamine, budget, queue pressure, and escalation signals.

5. Self-Documentation Updates
- Status: implemented (with significance gate)
- Where:
  - `systems/autonomy/self_documentation_closeout.ts`
  - `systems/spine/spine.ts` (`spine_self_documentation`)
- Notes: daily session summaries auto-upsert into `MEMORY.md`; significant shifts can require manual approval.

6. Evolution Tracking Dashboard
- Status: implemented
- Where:
  - `agent-holo-viz/server/system_visualizer_server.js`
  - `agent-holo-viz/client/app.js`
- Notes: commit velocity, churn, and stability trajectory exposed in visualizer.

## P2 (Design Phase)

7. Identity Delta Visualization
- Status: design pending
- Next step: define objective identity anchors (T1/T2, proposal-type mix, directive-fit trend) and render deltas over 7/30/90d.

8. Task Dependency Graph (IDE)
- Status: partial (backend dependency metadata exists)
- Next step: promote `proposal_dependencies` into dedicated graph view interactions (expand/collapse chains, blockers, reorder intent).

9. Autonomous Suggestion Mode (Pulsed)
- Status: partial (pulsed lane exists)
- Next step: add away-mode guard + low-priority notebook lane with explicit budget floor.

## Rejected Imports

Rejected by policy:
- Continuous background consciousness
- Self-writable constitution
- Self-replication via git
- Creator proposals as non-orders
- Percent-based background budget defaults
- Editable safety agent

Rationale: bounded autonomy and operator-owned alignment remain mandatory.
