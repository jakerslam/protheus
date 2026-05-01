# TODO Archive

Updated: 2026-05-01T22:34:40.407Z

## How To Use This File
- This is the historical ledger for completed work, not the live queue.
- Canonical structured data lives in [todo_archive_registry.json](/Users/jay/.openclaw/workspace/docs/workspace/todo/todo_archive_registry.json).
- The pre-JSON historical snapshot remains preserved at [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md).

## Historical Snapshot
- total_rows: 4145
- queued: 192
- in_progress: 10
- blocked: 2
- blocked_external_prepared: 32
- done: 1616
- existing_coverage_validated: 2302

## Scripted Completion Archive
- `KSENT-FRESHNESS-GUARD` — Enforce fresh Sentinel truth before using findings
  completed_at: `2026-05-01T22:34:40.385Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Prevent stale Sentinel artifacts from being treated as current truth by requiring freshness windows, generated_at checks, and stale_do_not_use labeling for old findings.
  completion_note: Implemented Kernel Sentinel current-truth freshness classification for final reports; stale findings are labeled stale_do_not_use, excluded from top_findings, and blocked from TODO/GitHub promotion. Validation: cargo test --manifest-path core/layer0/ops/Cargo.toml kernel_sentinel::report_budget -- --nocapture passed 5/5.
- `KSENT-SYMPTOM-CLUSTERS` — Cluster symptoms into root-cause families
  completed_at: `2026-05-01T21:48:36.512Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Group repeated UI/runtime/tool/eval findings into root-cause clusters so Sentinel detects structural failure families instead of isolated symptoms.
  completion_note: Implemented Kernel Sentinel root-cause symptom clustering for feedback rows, including structural surface-family cluster metadata, repeated-cluster promotion signals, SRS evidence, file-size split, and targeted Sentinel regression coverage.
- `KSENT-FEEDBACK-CONTRACT` — Add Sentinel feedback-to-TODO actionability contract
  completed_at: `2026-05-01T21:33:53.792Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Make each Sentinel feedback item declare whether it is todo_ready, triage_to_todo, or needs_root_cause_synthesis, with explicit evidence/actionability requirements.
  completion_note: Implemented Kernel Sentinel feedback-to-TODO actionability states (todo_ready, triage_to_todo, needs_root_cause_synthesis) with explicit evidence/actionability requirements, tests, and live auto-run feedback inbox smoke.
- `KSENT-FAILURE-LEVELS` — Strengthen Sentinel failure-level classification
  completed_at: `2026-05-01T21:26:03.140Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-07`
  source_family: `Kernel Sentinel Feedback Quality Upgrade`
  summary: Teach Sentinel to classify symptom, component, boundary, policy-truth, architectural, and self-model failures before recommending remediation.
  completion_note: Implemented Kernel Sentinel failure_class/review_depth classification, final-report failure_level_summary, and promotion candidate failure-level framing with targeted tests and live kernel-sentinel smoke.
- `WF-AUTH` — Finish workflow gate authority cleanup
  completed_at: `2026-05-01T21:15:11.122Z`
  previous_section: `red`
  owner: `unassigned`
  deadline: `2026-05-05`
  source_family: `Workflow Authority Cleanup`
  summary: Finish workflow gate authority cleanup so every turn enters Gate 1, tool/menu submissions are LLM-authored, and visible chat cannot be system-substituted.
  completion_note: Completed workflow gate authority cleanup wave: visible responses now carry explicit LLM-authored/system-substitution provenance, system fallback cannot be mislabeled as LLM-authored chat, and Gate 1 manual-toolbox candidate choices can promote to pending confirmation only from candidate-menu context.
- `KSENT-CANDIDATE-PIPELINE` — Stage Sentinel findings through a candidate pipeline before TODO or GitHub promotion
  completed_at: `2026-05-01T21:05:51.030Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Add a proposed-finding pipeline with todo_ready, issue_ready, needs_triage, and stale_do_not_use states so Sentinel drafts candidates first and Codex or a human approves promotion before the main TODO or GitHub is mutated.
  completion_note: Implemented Kernel Sentinel draft-only promotion lane: final reports now emit human-review-required promotion_candidates and triage_candidates with todo_ready/issue_ready/needs_triage states while forbidding Sentinel TODO/GitHub mutation or patch auto-apply.
- `KSENT-ROOT-CAUSE-CLUSTERING` — Cluster Sentinel symptoms into structural root-cause findings
  completed_at: `2026-05-01T20:58:04.031Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-15`
  source_family: `Sentinel Regression Audit May01`
  summary: Collapse repeated symptoms into shared structural issues when they point at the same boundary, ownership, policy, or runtime fracture instead of emitting parallel symptom-level findings.
  completion_note: Implemented Kernel Sentinel final-report root-cause clustering: release-ready findings are grouped by owner/category/root-frame/fingerprint family, top_findings carry cluster exemplar metadata, and root_cause_clusters summarize occurrence counts, sample IDs, evidence refs, and next actions.
- `KSENT-QUALITY-FILTER` — Add a strict release-quality filter for Sentinel findings
  completed_at: `2026-05-01T20:49:13.025Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Only release Sentinel findings that include evidence, recurrence or freshness support, owner guess, root-cause hypothesis, and a concrete next action; everything else stays in draft or triage state.
  completion_note: Implemented Kernel Sentinel release-quality filtering for final reports; top_findings now require trusted evidence, recurrence/freshness support, owner guess, root-cause hypothesis, and concrete next action, while weak observations stay bounded in triage_findings.
- `KSENT-REPORT-BUDGET` — Add Sentinel report-size budget and evidence compaction
  completed_at: `2026-05-01T20:40:40.512Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Keep Sentinel outputs bounded by separating raw evidence from operator summaries, adding report byte ceilings, and releasing only the useful compressed portion in the final report surface.
  completion_note: Implemented Kernel Sentinel byte-budgeted final report surface with compact top findings, raw evidence stream refs, CLI emission, regression tests, CLI smoke, and strict fail-closed validation.
- `KSENT-STALL-GUARD` — Make Sentinel auto-runs fail small instead of stalling
  completed_at: `2026-05-01T20:29:46.200Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `Sentinel Regression Audit May01`
  summary: Add timeout, heartbeat, and partial-artifact failover so ops:kernel-sentinel:auto either refreshes evidence or exits with a small diagnostic artifact instead of hanging silently.
  completion_note: Implemented Kernel Sentinel auto-run heartbeat/timeout guard with compact diagnostic artifact; validated timeout regression, direct CLI exit 124 artifact, and strict fail-closed Sentinel test.
- `CGPT-GOV-011` — Expose PrepareContext in plan displays and eval traces
  completed_at: `2026-05-01T18:16:18.278Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Make context preparation operator-visible whenever a read-like plan mutates session context so read/write semantics stay explicit.
  completion_note: Exposed PrepareContext/context_preparation in progress displays, package/candidate decision-trace metadata, and step trace inputs; conformance coverage asserts pure-read none and comparative explicit traces.
- `CGPT-GOV-007` — Keep tests/tooling as harness-only, not an ungoverned domain
  completed_at: `2026-05-01T18:10:12.050Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Ensure definition-shaped eval, benchmark, conformance, regression, release-gate, and scorecard truth lives under Validation/Observability while tests/tooling remains executor glue.
  completion_note: Added Validation-owned tooling harness-only policy and guard, moved six regression fixture matrices from tests/tooling/fixtures into validation/regression/fixtures, rewired fixture consumers, registered governance gates in the lifecycle registry, and validated positive/controlled-negative/tooling/test-maturity/transition/workspace/replay/churn paths.
- `CGPT-GOV-005` — Add tiered browser-rendered long-chat heap proof
  completed_at: `2026-05-01T17:53:35.874Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Add Tier 2 browser-rendered heap/DOM/storage regression and Tier 3 gateway-to-shell projection stress proof on top of the deterministic store-projection guard.
  completion_note: Added Validation-owned tiered long-chat heap proof contract and guard covering Tier 1 store-guard wiring, Tier 2 rendered Shell fixture budgets, and Tier 3 Gateway-to-Shell projection stress; positive, controlled-negative, transition, runtime-payload, JSON, line-cap, diff, and churn checks passed.
- `CGPT-GOV-004` — Compress chat.ts responsibilities
  completed_at: `2026-05-01T17:46:12.509Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Continue splitting chat.ts into projection rendering, input collection, detail fetch, and local display preferences while moving workflow, terminal, model, and tool coordination out.
  completion_note: Compressed chat.ts by extracting terminal prompt/cursor display math into chat_message_display_helpers.ts; delegated chat.ts and matching part file; dashboard build, helper smoke, Svelte shell contract, runtime payload budget, scoped diff check passed; long-chat RAM guard remains red on pre-existing Shell purge debt.
- `CGPT-GOV-003` — Runtime-check Shell session/message/event payload budgets
  completed_at: `2026-05-01T17:41:46.544Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Verify live session, messages, and websocket event paths conform to bounded Gateway projection contracts at runtime, not just in policy files.
  completion_note: Added runtime-shaped Shell payload budget contract and guard for session list, chat message window, and runtime event projections; validated against Gateway interface payload budgets with positive, controlled-negative, static budget, and transition-residue checks.
- `CGPT-GOV-002` — Prove all external surfaces enter through Gateways
  completed_at: `2026-05-01T17:36:46.228Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Audit Shell, CLI, SDK, issue-submission, eval-submission, and future app/mobile ingress so every external surface is Gateway-only with no first-party shell exception.
  completion_note: Added Validation-owned Gateway external surface contract and guard for Shell, CLI, SDK, issue/eval submission, plugin/external-agent, and future app/mobile surfaces; wired package/tooling/transition enforcement; positive guard passed and controlled negative failed closed.
- `CGPT-GOV-001` — Keep Tower metaphor out of active subsystem names
  completed_at: `2026-05-01T17:28:25.841Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-08`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Enforce that Tower remains historical/metaphor-only and cannot appear in active subsystem names, routes, generated maps, CI gates, or manifests.
  completion_note: Added ops:orchestration:naming:guard, registered it in tooling, wired transition-residue guard to require it, and proved positive plus controlled-negative enforcement.
- `CGPT-GOV-012` — Wire representation-collapse reporting into governance
  completed_at: `2026-05-01T05:34:09.811Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Track representation count for message, session, tool result, trace, workflow, task, evidence, and issue candidate so projections do not become new truth stores.
  completion_note: Wired representation-collapse reporting into package scripts and tooling registry with artifacts for representation risk triage.
- `CGPT-GOV-010` — Verify Shell amputation guard executable linkage
  completed_at: `2026-05-01T05:34:09.492Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Keep policy docs, package scripts, tooling registry, and actual shell-amputation guard path aligned so shell replaceability remains executable.
  completion_note: Verified and repaired Shell amputation executable linkage; guard now fixtures/scans orchestration instead of retired surface path.
- `CGPT-GOV-009` — Create Shell Cognition burn-down register
  completed_at: `2026-05-01T05:34:09.163Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Classify every Shell Cognition subsystem as presentation-local or re-home it to Orchestration, Observability, Validation, Kernel, Gateways, or compatibility debt.
  completion_note: Added Shell Cognition burn-down register covering all ten generated Shell Cognition subsystems with target domains and deadlines.
- `CGPT-GOV-008` — Retire or explicitly govern surface/orchestration
  completed_at: `2026-05-01T05:34:08.850Z`
  previous_section: `red`
  owner: `codex`
  deadline: `2026-05-10`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Define whether surface/orchestration is a compatibility mirror or active path, assign owner/expiry, and prevent drift from canonical orchestration/.
  completion_note: Governed retired surface/orchestration via transition register and rewired active package/guard references to orchestration/.
- `CGPT-GOV-006` — Create virtual-repo manifests for every top-level domain
  completed_at: `2026-05-01T05:34:08.529Z`
  previous_section: `yellow`
  owner: `codex`
  deadline: `2026-05-14`
  source_family: `ChatGPT Governance Audit Apr30`
  summary: Declare Kernel, Orchestration, Gateways, Shell, Validation, Observability, Governance, Conduit, Layer 3, and Adapters as virtual repos with owned contracts and dependency boundaries.
  completion_note: Implemented domain_virtual_repo_manifest.json and architecture_transition_residue_guard validation for required virtual repo domains.

## Legacy Appendix
- Preserved historical markdown: [TODO_ARCHIVE_LEGACY.md](/Users/jay/.openclaw/workspace/docs/workspace/todo/TODO_ARCHIVE_LEGACY.md)
