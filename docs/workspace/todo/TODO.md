# TODO

Updated: 2026-05-01T05:34:09.812Z

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
- active_items: 45
- red: 17
- yellow: 19
- white: 9

## Red Section (Do Immediately)
- `WF-AUTH` — Finish workflow gate authority cleanup
  owner: `unassigned`
  deadline: `2026-05-05`
  source_family: `Workflow Authority Cleanup`
  summary: Finish workflow gate authority cleanup so every turn enters Gate 1, tool/menu submissions are LLM-authored, and visible chat cannot be system-substituted.
- `KSENT-FAILURE-LEVELS` — Strengthen Sentinel failure-level classification
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Teach Sentinel to classify symptom, component, boundary, policy-truth, architectural, and self-model failures before recommending remediation.
- `KSENT-FEEDBACK-CONTRACT` — Add Sentinel feedback-to-TODO actionability contract
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Make each Sentinel feedback item declare whether it is todo_ready, triage_to_todo, or needs_root_cause_synthesis, with explicit evidence/actionability requirements.
- `KSENT-SYMPTOM-CLUSTERS` — Cluster symptoms into root-cause families
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Group repeated UI/runtime/tool/eval findings into root-cause clusters so Sentinel detects structural failure families instead of isolated symptoms.
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
- `CGPT-GOV-001` — Keep Tower metaphor out of active subsystem names
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Enforce that Tower remains historical/metaphor-only and cannot appear in active subsystem names, routes, generated maps, CI gates, or manifests.
- `CGPT-GOV-002` — Prove all external surfaces enter through Gateways
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Audit Shell, CLI, SDK, issue-submission, eval-submission, and future app/mobile ingress so every external surface is Gateway-only with no first-party shell exception.
- `CGPT-GOV-003` — Runtime-check Shell session/message/event payload budgets
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Verify live session, messages, and websocket event paths conform to bounded Gateway projection contracts at runtime, not just in policy files.
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
- `CGPT-GOV-004` — Compress chat.ts responsibilities
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Continue splitting chat.ts into projection rendering, input collection, detail fetch, and local display preferences while moving workflow, terminal, model, and tool coordination out.
- `CGPT-GOV-005` — Add tiered browser-rendered long-chat heap proof
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Add Tier 2 browser-rendered heap/DOM/storage regression and Tier 3 gateway-to-shell projection stress proof on top of the deterministic store-projection guard.

## Yellow Section (Do Soon)
- `CGPT-GOV-007` — Keep tests/tooling as harness-only, not an ungoverned domain
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Ensure definition-shaped eval, benchmark, conformance, regression, release-gate, and scorecard truth lives under Validation/Observability while tests/tooling remains executor glue.
- `CGPT-GOV-011` — Expose PrepareContext in plan displays and eval traces
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Make context preparation operator-visible whenever a read-like plan mutates session context so read/write semantics stay explicit.
- `KSENT-ACTION-SYNTHESIS` — Improve issue/TODO synthesis specificity
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Require component, observed failure, root-cause hypothesis, repair type, acceptance criteria, validation route, and evidence before promoting feedback to TODOs.
- `KSENT-ANTI-PATCHING` — Detect symptom-patching loops
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Flag repeated patches that change visible symptoms while the same boundary, policy, or ownership violation remains unresolved.
- `KSENT-AUTHORITY-GHOSTS` — Detect authority ghosts after refactors
  owner: `sentinel`
  deadline: `2026-05-14`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Add first-class authority_ghost findings for projection layers, caches, shims, or adapters that preserve old authority shape after syntax-level cleanup.
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
