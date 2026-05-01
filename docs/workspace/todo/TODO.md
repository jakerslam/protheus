# TODO

Updated: 2026-05-01T22:58:52.384Z

## How To Use This File
- This is the live operating board, not the historical ledger.
- Canonical structured data lives in [todo_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_registry.json).
- Completed items must be moved to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE.md) instead of being left here.
- Archive history is rendered from [todo_archive_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_archive_registry.json) and the preserved legacy appendix at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md).
- Run manual commands through `npm run -s ops:todo:board -- <command>` so JSON and Markdown stay in sync.
- Every active item must declare `owner` and `deadline`.
- Allowed deadline values: exact date like `2026-05-07`, `none`, `external`, or dependency-shaped values like `after_red_section`.
- Deadline promotion policy: items due in <= 7 days belong in Red; items due in <= 14 days belong in Yellow; everything later stays in White unless manually escalated.

## Rollup
- active_items: 36
- red: 8
- yellow: 19
- white: 9

## Red Section (Do Immediately)
- `SHELL-CLEANUP` — Finish the Shell source-of-truth cleanup
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Cleanup Wave`
  summary: Finish the Shell source-of-truth cleanup by closing the duplicate TS inventory and breaking chat.ts into canonical modules instead of mirrored artifacts.
- `SHELL-EXTRACT-CHAT` — Move chat and session projections behind Gateway
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make chat/session list and message-window loading available as bounded Gateway projections with CLI/headless proof, so the Shell no longer needs full conversation ownership.
- `SHELL-EXTRACT-COMMANDS` — Move interactive command authority out of Shell
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Route slash commands, model switching, model failover, prompt queue execution, and terminal execution through typed Gateway ingress receipts instead of browser-owned helpers.
- `SHELL-EXTRACT-DETAILS` — Move message and tool details behind lazy routes
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Expose message, tool, artifact, trace, and workflow detail fetches by stable refs through bounded audited Gateway routes instead of default Shell payloads.
- `SHELL-EXTRACT-MATRIX` — Prove the headless capability matrix
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Create the headless capability matrix and prove which Shell-visible operations already work through CLI/Gateway without browser assets before touching legacy dashboard code.
- `SHPURGE` — Finish the Shell authority purge
  owner: `unassigned`
  deadline: `2026-05-07`
  source_family: `Shell Authority Purge Completion Backlog`
  summary: Finish the Shell authority purge so the Shell becomes projection/input only and stops acting like a shadow runtime.
- `SRS-ACTIVE` — Keep active SRS intake items moving
  owner: `unassigned`
  deadline: `2026-05-09`
  source_family: `Actionable SRS Items (Queued/In Progress)`
  summary: Keep active SRS intake items moving before opening more new fronts.
- `ALPINE-PURGE` — Remove the remaining Alpine boot/runtime dependency
  owner: `unassigned`
  deadline: `2026-05-10`
  source_family: `Shell Alpine Purge Wave`
  summary: Remove the remaining Alpine boot/runtime dependency once the retirement guard is green.

## Yellow Section (Do Soon)
- `KSENT-CONTRADICTIONS` — Add policy-vs-runtime contradiction detection
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Compare doctrine, contracts, code paths, and artifacts to surface semantic contradictions such as projection-only policy with runtime-state mirrors.
- `KSENT-FRESHNESS-TIERS` — Separate current truth from stale Sentinel reference
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Classify Sentinel outputs as current_live_truth, recent_but_not_current, historical_trend, or stale_reference_only before using them for decisions.
- `KSENT-UNDERSTANDING-WORKSHEET` — Generate recurring system-understanding worksheets
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Have Sentinel periodically produce a self-understanding dossier covering soul, runtime behavior, authority map, boundary map, drift, gaps, and confidence.
- `SHELL-EXTRACT-CACHE` — Replace Shell conversation cache and search with projections
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Move conversation cache/search to bounded preview/index routes so the browser keeps only cursors, previews, counts, and refs rather than cloned full messages or raw tool payloads.
- `SHELL-EXTRACT-EVAL` — Move report/eval issue flow behind Gateway ingress
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make chat-local report issue send bounded refs/context through Gateway to Orchestration eval without Shell-owned policy, GitHub submission, or raw context upload.
- `SHELL-EXTRACT-HEALTH` — Make health and connectivity purely Gateway projected
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make runtime health, connectivity, release status, and degraded-state labels come from bounded Gateway status projections so Shell does not infer readiness or failure truth.
- `SHELL-EXTRACT-LIFECYCLE` — Move agent and session lifecycle to receipts
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Ensure create, select, archive, reset, retry, and new-agent initialization flows work through Gateway/Core/Orchestration receipts with headless proof.
- `SHELL-EXTRACT-WORKFLOW` — Project workflow stage and thought status from Orchestration
  owner: `unassigned`
  deadline: `2026-05-14`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Make Orchestration emit workflow_stage_label and workflow_thought_preview projections so thinking bubbles render owner-provided status instead of Shell-authored fallback text.
- `KSENT-FINAL-REPORT` — Publish Sentinel final output docs for top findings
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Emit compact final output docs that summarize release blockers, top findings, evidence refs, and next actions instead of forcing operators to inspect huge raw state artifacts.
- `KSENT-ISSUE-BOUNDEDNESS` — Triage Sentinel boundedness findings into concrete repair work
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Turn the workspace tooling boundedness findings into a concrete repair lane with owner, acceptance criteria, and replay/validation commands.
- `KSENT-ISSUE-EMPTY-RESPONSES` — Triage synthetic empty-response failures from Sentinel
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Convert repeated synthetic user harness empty_assistant_response findings into a concrete issue lane covering routing, finalization, and synthesis failure causes.
- `KSENT-ISSUE-RELEASE-BRIDGES` — Repair Sentinel release-evidence and receipt-integrity blockers
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Investigate and close the release_evidence and receipt_integrity bridge failures that currently drive Kernel Sentinel release_fail verdicts.
- `OBS-CURRENT-TRUTH-GUARD` — Strengthen Observability current-truth vs stale-reference handling
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Treat stale observer artifacts as an Observability-layer problem by enforcing current_live_truth vs historical_reference semantics across Sentinel and related evidence consumers.
- `TRACE` — Finish the universal trace substrate contract
  owner: `unassigned`
  deadline: `2026-05-16`
  source_family: `Universal Trace Substrate / Fragmented Observability Closure`
  summary: Finish the Observability-owned universal trace substrate contract and anti-fragmentation guard.
- `WF-UTILITY` — Build the workflow utility spine
  owner: `unassigned`
  deadline: `2026-05-20`
  source_family: `Workflow Utility Spine After Shell Purge`
  summary: Build the workflow utility spine so the system is useful for real work after Shell de-authority.
- `TRACE-IMPL` — Implement end-to-end unified trace_id propagation
  owner: `unassigned`
  deadline: `2026-05-23`
  source_family: `Universal Trace Runtime Implementation Checklist`
  summary: Implement end-to-end unified trace_id propagation from initial request through Orchestration, workflows, tools, Kernel receipts, Sentinel, and final response.
- `MLSYS5` — Assimilate MLSysBook Chapter 5 systems lessons
  owner: `unassigned`
  deadline: `2026-05-27`
  source_family: `MLSysBook Vol. 1 Chapter 5 Neural Computation Implementation Backlog`
  summary: Assimilate Chapter 5 systems lessons into workload awareness, confidence routing, budgeting, and D-A-M diagnosis.
- `ARCH-TOOLING-NEXT` — Hold important architecture and tooling deltas behind current closure work
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Architecture and Tooling Follow-On Wave`
  summary: Important architecture and tooling deltas should follow the Shell, workflow, and trace closure work rather than compete with it immediately.
- `SRS-NEXT` — Queue the next SRS stream after the red section
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Actionable SRS Items (Queued/In Progress)`
  summary: Queue the next SRS stream after the red intake set so the active SRS flow stays coherent instead of fragmenting into too many parallel themes.

## White Section (Do At Leisure)
- `KSENT-BIGPICTURE` — Add big-picture regression mode
  owner: `sentinel`
  deadline: `after_red_section`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: When many subsystem symptoms co-occur, have Sentinel pause local-ticket generation and emit a structural diagnosis with recommended rebuild/realignment mode.
- `KSENT-SELF-REVIEW` — Add Sentinel feedback quality self-review
  owner: `sentinel`
  deadline: `after_red_section`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Track whether Sentinel findings were accepted, rejected, actionable, resolved, or merely symptom patches so feedback quality improves over time.
- `SHELL-NEXT` — Build Shell-next only after authority extraction proofs
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Shell Authority Extraction Without Legacy Shell Mutation`
  summary: Start the clean Shell-next route as a projection/input-only UI after the headless capability matrix and high-risk authority extraction routes are proven.
- `SVELTE` — Keep remaining Svelte migration subordinate to Shell authority purge
  owner: `unassigned`
  deadline: `after_red_section`
  source_family: `Chat Dashboard Alpine to Svelte Migration Wave`
  summary: Remaining Svelte migration and memory profiling items are worth doing, but they are subordinate to the broader Shell authority purge.
- `EXTERNAL-BLOCKERS` — Keep externally blocked work parked in the archive
  owner: `unassigned`
  deadline: `external`
  source_family: `External Blockers`
  summary: External blockers from the previous ledger remain parked in the archive until the required evidence packets or human approvals exist.
- `ASSIMILATION-LONGHORIZON` — Park longer-horizon assimilation and runtime work
  owner: `unassigned`
  deadline: `none`
  source_family: `Long-Horizon Assimilation and Runtime Work`
  summary: Assimilation and longer-horizon runtime work can stay parked here until the red and yellow closure work is materially better.
- `BACKLOG-PARKED` — Keep lower-pressure valid backlog items parked
  owner: `unassigned`
  deadline: `none`
  source_family: `Parked Valid Backlog Items`
  summary: These are still valid backlog items, but they are not the current forcing function.
- `OS` — Keep Layer 3 and OS-readiness work parked behind current closure work
  owner: `unassigned`
  deadline: `none`
  source_family: `OS-Readiness Wave - Layer 3 to True OS Migration`
  summary: Layer 3 and OS-readiness work remains important, but it should stay behind the current Shell, workflow, and trace closure push.
- `TAURI-UI` — Keep Tauri migration strategically queued
  owner: `unassigned`
  deadline: `none`
  source_family: `Tauri Desktop App Migration + Memory Fix Wave`
  summary: Tauri migration remains strategically valuable, but it should not outrun the deeper architecture cleanup already underway.

## Archive Rule
- When an item in this file is completed, remove it from this live board and append it to [TODO_ARCHIVE.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE.md) through the scripted flow.
- Do not let completed rows accumulate here again.
- Treat Markdown as a rendered operator surface, not the canonical mutation target.
