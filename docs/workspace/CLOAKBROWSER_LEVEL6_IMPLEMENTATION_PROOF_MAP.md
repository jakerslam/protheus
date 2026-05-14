# CloakBrowser Level 6 Implementation Proof Map

Created: 2026-05-14

Source assimilation ledger: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Level 5 syntax map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md`

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Purpose

Level 6 is the implementation/proof stage. Level 5 closed the CloakBrowser file-by-file assimilation queue; Level 6 turns the extracted contracts into an Infring-native browser materialization primitive and proves whether it actually improves web retrieval quality.

This is not more CloakBrowser source assimilation. It is a controlled implementation campaign:

```text
browser materialization contract
-> fake provider proof
-> local live provider proof
-> extraction/evidence packaging
-> web-tooling gates
-> research workflow consumption
-> golden/live quality measurement
```

## Level 6 Rule

No Level 6 slice is complete until it has all five statuses:

```text
implementation target mapped
-> code or explicit rejection landed
-> mock/local tests passed
-> diagnostics/gates visible
-> measured against research workflow impact
```

If a slice cannot be measured yet, it must be marked `deferred` with the missing prerequisite. Do not claim retrieval quality improvement from contracts alone.

## Guardrails

- Browser materialization remains default-off and capability-admitted.
- Ordinary search/fetch providers remain the first-line path.
- Browser execution may not become hidden default search behavior.
- Workflow CDs may request an admitted capability outcome, but must not receive browser handles, raw CDP URLs, raw launch args, raw scripts, raw HTML, screenshots, console logs, cookies, storage state, or local profile paths.
- Browser materialization output must re-enter the evidence pack before synthesis sees it.
- Local fixture tests come before live internet tests.
- Anti-bot bypass claims require evidence. A local materializer passing tests is not proof of bot-wall solving.
- Do not add proxy, persistent sessions, humanized interaction, external browser agents, or service pools until the one-shot stateless materializer is proven.

## Current Starting Point

| Surface | Current State | Level 6 Role |
| --- | --- | --- |
| Tool CD | `web_retrieval_v0.tool.json` declares `browser_materialize_page`, profile contracts, output contracts, dependency lifecycle, and adapter handoff rules. | Keep as the capability declaration. Extend only for implementation-required fields. |
| Policy | Browser materialization is default-off with safety, profile, readiness, and evidence contracts. | Use as the runtime source of truth. Do not hardcode behavior elsewhere. |
| API | `api_browser_materialize_page` validates URL/admission and fails closed when disabled or no adapter is ready. | Add fake/local provider execution behind the same boundary. |
| CLI | `web-conduit browser-materialize` routes through the API. | Use as a proof harness for local fixture pages. |
| Tests | Mock-fast browser materialization contract tests pass. | Add fake provider, local fixture, extraction, and evidence promotion tests. |
| Evidence | Handoff contract exists, but no real materialized page evidence is produced yet. | Convert materialized output into evidence candidates and quality diagnostics. |
| Workflow | Research workflow can see web tooling results and gates. | Consume materialized evidence without exposing adapter internals. |

## Implementation Burn-Down

Status values:

- `pending`: not started.
- `in_progress`: current focused implementation slice.
- `integrated`: code and docs landed with tests.
- `rejected`: intentionally not implemented.
- `deferred`: useful but blocked by an explicit prerequisite.

| Order | Slice | Status | Primary Question | Likely Infring Target |
| ---: | --- | --- | --- | --- |
| 1 | Fake materialization provider | integrated: deterministic provider proof pass 001 | Can the existing API emit a valid materialized-page object without launching a browser? | `core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` and browser materialization tests. |
| 2 | Fixture artifact quarantine | integrated: ref-only artifact proof pass 002 | Are raw HTML/screenshot-like payloads stored by ref and never chat-visible? | Web conduit artifact/quarantine helpers and tests. |
| 3 | Evidence candidate conversion | pending | Can a materialized page become evidence-pack candidates with title, final URL, text, links, claim hints, score, quality flags, and refs? | Batch query/evidence pack pipeline plus Tool CD output contract. |
| 4 | Local static page provider proof | pending | Can the adapter fetch a local fixture page through the materialization API with full cleanup? | Browser adapter helper, CLI proof, local fixture tests. |
| 5 | Local JS-rendered page proof | pending | Does browser materialization recover content that direct fetch cannot see? | Local fixture server plus browser materialization integration test. |
| 6 | Redirect and final URL safety proof | pending | Does final URL revalidation block unsafe redirect targets before extraction? | SSRF/final URL guard tests. |
| 7 | Timeout/blocker classification | pending | Can the adapter classify timeout, access denied, anti-bot shell, JS-required, and content-too-thin separately? | Web tooling diagnostics and materialization result shape. |
| 8 | Web tooling gate split | pending | Can tooling stats isolate readiness, URL safety, materialization, extraction, evidence promotion, and synthesis consumption? | Web retrieval gate diagnostics/eval reporting. |
| 9 | Research workflow consumption | pending | Does the research CD consume materialized evidence as normal evidence rather than tool trace text? | Research workflow CD/eval path; no prompt hardcoding. |
| 10 | Golden/live impact pass | pending | Does the primitive improve weak-data cases without regressing upstream gates? | Research golden eval, web tooling eval, failure archive. |

## Slice Details

### L6-001 Fake Materialization Provider

Goal: prove the runtime path can produce a complete materialized page object through the existing API without introducing browser dependency risk.

Required behavior:

- Runs only when the capability is explicitly enabled in test policy.
- Returns deterministic fixture output with source URL, final URL, title, main text, links, blocker class, extraction confidence, cleanup status, artifact refs, and evidence handoff state.
- Preserves existing fail-closed behavior when disabled or adapter is not ready.

Exit tests:

- default-off still fails closed,
- ready fake adapter returns valid output,
- raw payload is absent from chat-visible fields,
- final URL safety is present,
- cleanup status is present.

Status: integrated in pass 001.

Implemented:

- Added a CD-selected `fake_materialization` provider path behind the existing `api_browser_materialize_page` boundary.
- Preserved default-off and normal ready-local-browser stub behavior; the fake provider only runs when policy enables browser materialization, marks the adapter ready, and selects `fake_materialization` in `provider_order`.
- Returned a deterministic materialized-page object with source URL, final URL, title, extracted text, link summary, blocker classification, extraction confidence, readiness strategy, cleanup status, retry diagnostics, artifact ref, and evidence-candidate shell.
- Kept `browser_launch_attempted=false` and raw payload visibility false because this slice proves contract execution shape, not live browser execution.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL6_IMPLEMENTATION_PROOF_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Impact:

This does not improve live retrieval yet. It proves that the materialization API can now produce a complete success-shaped artifact through policy/CD selection without using raw browser execution or contaminating ordinary search.

### L6-002 Fixture Artifact Quarantine

Goal: prove raw page material is stored or referenced as an artifact, never projected into final chat or workflow traces.

Required behavior:

- Raw HTML is represented by an artifact ref.
- Screenshots are either absent or represented by bounded artifact refs.
- Console/network/browser trace content stays telemetry-only.
- Evidence candidates receive extracted text/metadata, not raw browser payloads.

Exit tests:

- raw HTML is not present in response projection,
- artifact ref is present when raw page material exists,
- redaction/visibility guards reject direct raw payload projection.

Status: integrated in pass 002.

Implemented:

- Added a ref-only artifact manifest to the fake materialization provider.
- Represented raw HTML, extracted text, and browser trace material as artifact refs with raw bytes hidden from chat and workflow traces.
- Kept screenshot, console log, and network log payloads absent unless a future adapter explicitly captures them by ref.
- Added artifact quarantine fields that prove raw artifacts are not projected and evidence receives extracted text only.
- Rejected caller-supplied raw HTML, screenshot bytes, browser traces, console logs, and network logs before any provider execution.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json docs/workspace/CLOAKBROWSER_LEVEL6_IMPLEMENTATION_PROOF_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Impact:

This still does not improve live retrieval, but it closes the next proof obligation: materialized page internals now have an explicit quarantine shape before evidence promotion work begins.

### L6-003 Evidence Candidate Conversion

Goal: bridge browser output into the same evidence-pack lane as search/fetch results.

Required evidence fields:

- source kind/class,
- title,
- final/source URL,
- source domain,
- extracted snippet or text summary,
- claim hints,
- term hints,
- score components,
- confidence,
- quality flags,
- coverage facets,
- freshness/timestamp,
- permissions,
- artifact refs.

Exit tests:

- materialized page creates evidence-pack candidates,
- evidence quality is `usable`, `low_signal`, or `rejected` based on extracted content,
- blocker shells do not promote as source truth,
- evidence refs are available to synthesis.

### L6-004 Local Static Page Provider Proof

Goal: prove one admitted local browser/materializer path can retrieve and extract a simple fixture page.

Implementation preference:

- Use the narrowest adapter available in the repo/runtime.
- Prefer a helper behind `web_conduit` rather than workflow Rust.
- If a Node/Playwright helper is used, Rust remains the contract executor and receives structured JSON only.

Exit tests:

- local page materializes,
- title/text/links are extracted,
- cleanup runs on success and failure,
- no raw browser handle or CDP URL is visible.

### L6-005 Local JS-Rendered Page Proof

Goal: prove browser materialization gives us something ordinary fetch cannot: rendered content after bounded readiness.

Exit tests:

- direct fetch lacks rendered text,
- browser materialization captures rendered text,
- readiness strategy is policy-owned,
- no caller-supplied script is accepted.

### L6-006 Redirect And Final URL Safety Proof

Goal: ensure browser execution does not weaken the fetch SSRF safety boundary.

Exit tests:

- safe initial URL redirecting to private/internal host is blocked before extraction,
- credentialed final URL is blocked,
- non-HTTP(S) final URL is blocked,
- final URL safety result is projected in diagnostics.

### L6-007 Timeout And Blocker Classification

Goal: make materialization failures measurable instead of collapsing into "bad results."

Blocker classes:

- `none`,
- `timeout`,
- `access_denied`,
- `anti_bot_challenge`,
- `needs_js`,
- `rate_limited`,
- `content_too_thin`,
- `adapter_not_ready`,
- `unsafe_url`,
- `extraction_failed`.

Exit tests:

- fixture pages map to expected blocker classes,
- blocker class affects evidence promotion,
- retry recommendations are telemetry-only and budget-aware.

### L6-008 Web Tooling Gate Split

Goal: isolate the web tooling bottleneck with gates that say where retrieval failed.

Gate candidates:

- `web_gate_1_provider_ready`,
- `web_gate_2_candidate_url_safety`,
- `web_gate_3_materialization_attempted`,
- `web_gate_4_materialization_result`,
- `web_gate_5_extraction_quality`,
- `web_gate_6_evidence_promotion`,
- `web_gate_7_synthesis_consumed_evidence`.

Exit tests:

- gates are emitted for fake/local materialization,
- failures classify hard vs. soft,
- gates are visible in workflow reports without leaking raw payloads.

### L6-009 Research Workflow Consumption

Goal: make the research CD consume materialized evidence through the same synthesis path as other evidence.

Required behavior:

- No research-domain hardcoding.
- No specific output format hardcoding.
- No direct tool trace in final answer.
- Materialized evidence is one evidence source among others.
- Low-evidence fallback still works when materialization is blocked or low-signal.

Exit tests:

- synthesis sees evidence refs from materialized output,
- final answer does not mention internal gate names/tool traces,
- verifier can reject answers that ignore materialized evidence.

### L6-010 Golden And Live Impact Pass

Goal: measure whether browser materialization improves real user-facing research quality.

Required measurement:

- gate pass rate,
- golden pass rate,
- excellent/average rolling metrics,
- hard vs. soft failure archive,
- per-domain retrieval quality,
- browser lane attempted/ready/materialized/promoted counts.

Exit criteria:

- No upstream workflow gate regression,
- no raw tool leakage,
- no empty responses,
- web tooling failures are diagnosable by gate,
- at least one weak retrieval case improves because materialized evidence enters synthesis.

## Deferred Capability Map

| Capability | Deferred Until | Reason |
| --- | --- | --- |
| Proxy support | Stateless local materializer passes L6-001 through L6-010. | Permission-sensitive and secret-bearing. |
| Persistent sessions | Stateless materializer proves value and cleanup. | Identity retention and storage lifecycle risk. |
| Humanized interaction | Read-only materialization cannot retrieve enough dynamic content. | Interaction expands policy and abuse surface. |
| External browser-use agents | Core materializer and evidence packaging are stable. | Agent loops are a separate workflow/tool class. |
| CDP service pool | Single-shot adapter is too slow but useful. | Pooling expands authority, lifecycle, and cleanup surface. |
| Live anti-bot site tests | Local fixture tests and policy gates are stable. | External sites are flaky and can distort release confidence. |

## Target Files

Likely implementation targets:

- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs`
- `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/**`
- `/Users/jay/.openclaw/workspace/orchestration/src/eval_web_retrieval_gate_diagnostics.rs`
- `/Users/jay/.openclaw/workspace/orchestration/src/eval_research_golden*.rs`
- `/Users/jay/.openclaw/workspace/orchestration/src/control_plane/workflows/official/research_synthesize_verify.workflow.json`

Potential adapter targets must be chosen during L6-004:

- a narrow Rust-owned helper that invokes a browser adapter,
- a Node/Playwright helper returning structured JSON,
- or an existing local browser capability if already present.

The decision must preserve Rust/CD authority: Tool CD and policy own allowed behavior; adapter code only performs admitted execution and returns structured observations.

## Validation Ladder

Run validation in this order as slices land:

1. `cargo fmt --check`
2. `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
3. `git diff --check -- <touched files>`
4. `cargo test -p infring-ops-core browser_materialization --lib`
5. targeted evidence-pack tests for materialized output
6. targeted web retrieval gate diagnostics tests
7. research golden subset for weak retrieval cases
8. full research workflow/golden pass

Do not run broad live external tests until local fixture tests pass.

## Exit Criteria

Level 6 is complete when:

- A fake provider and a local live provider both run behind the existing browser materialization API.
- Browser materialization can extract static and JS-rendered fixture content.
- Final URL safety is enforced after navigation.
- Raw page/browser payloads are artifact refs only and never chat-visible.
- Materialized output becomes evidence-pack candidates with quality flags and source refs.
- Web tooling reports isolate provider readiness, URL safety, materialization, extraction, and evidence-promotion failures.
- The research workflow can synthesize from materialized evidence without workflow-specific hardcoding.
- Golden/live stats show whether the primitive improves weak retrieval cases.

Level 6 is not complete if the only result is more contracts. It must prove useful retrieval movement.
