# CloakBrowser Level 3 Mechanics / Algorithm Map

Created: 2026-05-14

Source assimilation ledger: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Level 2 contract map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL2_BEHAVIORAL_CONTRACT_MAP.md`

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Purpose

Level 3 turns the Level 2 contracts into concrete mechanics and algorithms. The goal is to specify how web tooling should decide what happened, what to try next, what to promote to evidence, and what to keep out of the user-visible answer.

This document is still pattern assimilation, not source copying. It should guide implementation while keeping browser, proxy, persistent session, humanized interaction, and service-pool behavior gated and optional.

## Level Stack

| Level | Name | Question Answered | Current Artifact |
| --- | --- | --- | --- |
| 1 | Architecture / pattern | What patterns are useful and where do they belong? | CloakBrowser assimilation ledger |
| 2 | Behavioral contract | What must the system accept, reject, classify, expose, hide, and prove? | Level 2 contract map |
| 3 | Mechanics / algorithm | How should the system classify, decide, retry, extract, score, and promote? | This document |
| 4 | Implementation structure | Which modules, files, CD fields, adapters, and tests own each behavior? | Level 4 implementation structure map |
| 5 | Syntax | Exact code, branches, fields, regexes, and assertions. | Last wave |

## Guardrails

- Mechanics must stay domain-general. They cannot assume software research, Infring comparisons, benchmark prompts, or specific answer formats.
- The agent remains responsible for query choices and synthesis. The tool may provide strategy signals, not hidden user-intent assumptions.
- Browser materialization is a recovery/enrichment lane, not the default search path.
- Proxy, session, humanized interaction, and service pooling remain separately admitted capabilities.
- Runtime code may classify tool results and enforce safety, but must not invent research workflow behavior.
- Raw browser internals, proxy details, cookies, console logs, network traces, and raw HTML remain telemetry/artifact-only unless converted into evidence and synthesized.
- Evidence promotion is the control point: browser success, fetch success, or search success is not source truth by itself.

## Mechanics Overview

```text
user goal
-> agent-submitted query or query pack
-> provider result normalization
-> candidate filtering and blocker signal extraction
-> evidence candidate scoring
-> decision lattice
   -> synthesize from evidence
   -> agent refine query pack
   -> direct fetch/materialize candidate
   -> browser materialize candidate, if admitted
   -> alternate provider, if available
   -> structured low-evidence state
-> evidence pack handoff
-> synthesis
```

## Algorithm Targets

| ID | Mechanic | Purpose | CloakBrowser Pattern | Infring Target | Status |
| --- | --- | --- | --- | --- | --- |
| `CLOAK-L3-001` | Provider result normalization | Convert heterogeneous provider output into one candidate/error model. | Playwright/Puppeteer wrapper parity | batch-query/web-conduit normalization | partial |
| `CLOAK-L3-002` | Blocker signal extraction | Detect access/render/provider blockers before weak data is treated as evidence. | Anti-detection and Lambda failure tests | web quality diagnostics | integrated |
| `CLOAK-L3-003` | Retrieval decision lattice | Choose synthesize/retry/fetch/browser/alternate/low-evidence based on evidence state. | Lambda retry classification | retrieval broker diagnostics | integrated |
| `CLOAK-L3-004` | Query refinement signal generation | Help the agent issue better query packs from low-signal results without hardcoding prompts. | Failure-class-driven retry strategy | batch-query strategy hints | integrated |
| `CLOAK-L3-005` | URL safety mechanics | Validate initial and final URLs around any materialization attempt. | Lambda SSRF/redirect checks | Gateway/web-conduit safety | diagnostics integrated |
| `CLOAK-L3-006` | Profile compilation mechanics | Build one deterministic browser launch/profile artifact from policy. | `args.ts` and context option filtering | Tool CD profile compiler | diagnostics integrated |
| `CLOAK-L3-007` | Page settle/readiness mechanics | Decide when a browser-materialized page is ready to extract. | Lambda smart waits and humanized readiness ideas | future browser materializer | diagnostics integrated |
| `CLOAK-L3-008` | Main content extraction mechanics | Convert rendered page state into bounded text/markdown candidates. | Browser DOM/readability extraction pattern | evidence candidate enrichment | diagnostics integrated |
| `CLOAK-L3-009` | Evidence promotion scoring | Promote only relevant, substantive, safe, source-classified content. | Extracted page output discipline | evidence pack | integrated |
| `CLOAK-L3-010` | Retry budget and stop conditions | Prevent loops, provider thrash, and repeated weak-result cycling. | Bounded retry patterns | batch-query/retrieval gates | diagnostics integrated |
| `CLOAK-L3-011` | Readiness lifecycle mechanics | Report missing/installed/degraded/cleanup states without surprise installs. | Binary/cache/update lifecycle | web-conduit status | diagnostics integrated |
| `CLOAK-L3-012` | Artifact quarantine mechanics | Keep raw payloads accessible by ref, not chat-visible. | Service/browser trace separation | artifact store and telemetry | diagnostics integrated |
| `CLOAK-L3-013` | Mock-fast mechanics tests | Prove decisions with fixtures before live browser/runtime work. | Unit tests around launch/proxy/security | ops tests | integrated |

## Level 3 Assimilation Wave 1: Decision Lattice And Query Signals

Status: integrated.

Implemented targets:

- `CLOAK-L3-002`: blocker taxonomy is now consumed as a first-class retrieval decision input.
- `CLOAK-L3-003`: web quality diagnostics emit `retrieval_decision_lattice_v1`.
- `CLOAK-L3-004`: retry diagnostics emit `query_refinement_signals_v1`.
- `CLOAK-L3-013`: mock-fast tests cover blocker recovery, direct fetch, weak-source refinement, degraded provider fallback, and ready-for-synthesis paths.

Implementation files:

- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`

Mechanics now represented in diagnostics:

- `synthesize_from_evidence` when coverage is complete and retry is not recommended.
- `direct_fetch_candidate` when a candidate URL exists but no evidence has been promoted.
- `browser_materialize_candidate` only when a concrete candidate URL exists and the blocker is render/access/content materialization related.
- `alternate_provider` when the blocker is provider-level, throttling, anti-bot, JavaScript-required without a candidate URL, or access boundary related.
- `agent_refine_query_pack` when the issue is query/relevance/evidence shape, such as weak single-source or comparison coverage gaps.
- `structured_low_evidence` as the terminal low-evidence fallback when no higher-value retrieval action remains.

Important boundary:

This wave does not generate hidden queries. It exposes terms, missing facets, blocker class, and strategy signals so the agent can choose the next query pack. It also does not add a live browser executor; browser materialization remains a separately admitted capability.

## Level 3 Assimilation Wave 2: Evidence Promotion And Provider Normalization

Status: integrated.

Implemented targets:

- `CLOAK-L3-001`: provider attempts now emit a `provider_normalization_v1` broker report with status, phase, raw-row, synthesis-row, low-confidence-row, and failure-class counts.
- `CLOAK-L3-005`: evidence promotion now records URL safety hints for non-HTTP, credentialed, and internal-host candidate locators.
- `CLOAK-L3-009`: evidence pack rows now include `evidence_promotion_v1`, explaining promotion decision, safety state, scoring components, caveats, and chat-visibility boundary.
- `CLOAK-L3-013`: mock-fast tests cover provider normalization metadata and caveated promotion of unsafe/internal locators.

Implementation files:

- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`

Mechanics now represented in diagnostics:

- Evidence rows distinguish `promoted`, `promoted_with_caveats`, and `retained_low_confidence`.
- Promotion metadata includes query overlap, content richness, claim-hint count, coverage-facet count, source trust delta, freshness status, score, blocker absence, permissions, and caveats.
- Provider normalization is separate from evidence quality, so a run can show provider degradation, raw rows filtered away, low-confidence rows retained, or usable synthesis candidates without collapsing those states into one low-signal bucket.

Important boundary:

This wave does not turn URL-safety hints into the full Gateway browser-materialization safety gate. It makes safety assumptions visible at evidence-promotion time. Full pre-navigation, redirect, DNS/IP, and final-URL enforcement remains a lower-level browser/materialization implementation task.

## Level 3 Assimilation Wave 3: Retry Stop Conditions And Artifact Quarantine

Status: integrated as diagnostics.

Implemented targets:

- `CLOAK-L3-010`: retrieval broker diagnostics now emit `retry_stop_conditions_v1`, separating continue/stop decisions from raw provider quality.
- `CLOAK-L3-012`: retrieval broker diagnostics now emit `artifact_quarantine_v1`, showing that raw provider and evidence artifacts stay out of chat-visible payloads.
- `CLOAK-L3-013`: mock-fast tests now cover alternate-provider continuation, ready-for-synthesis stop state, raw artifact quarantine, and evidence promotion projection.

Implementation files:

- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`

Mechanics now represented in diagnostics:

- `stop_ready_for_synthesis` when evidence is ready for final synthesis.
- `stop_structured_low_evidence` when the low-evidence lane is the terminal, budgeted state.
- `continue_with_alternate_provider_if_admitted`, `continue_with_browser_materialization_if_admitted`, `continue_with_direct_fetch_if_budget_remains`, and `continue_with_agent_query_refinement_if_budget_remains` when another retrieval move is still useful.
- `stop_or_escalate_after_retry_budget` when query refinement is already exhausted.
- Raw payload and artifact references are counted and reported as quarantined rather than projected as final-answer material.

Important boundary:

This wave does not enforce retry execution or cleanup policy by itself. It makes the broker's continuation and quarantine state explicit so workflow gates can diagnose whether retrieval stopped for a good reason.

## Level 3 Assimilation Wave 4: URL Safety, Profile Contract, And Extraction Readiness

Status: integrated as diagnostics.

Implemented targets:

- `CLOAK-L3-005`: candidate URLs now receive `url_safety_assessment_v1` with scheme, credentials, internal-host, rejection reasons, and redirect revalidation requirements.
- `CLOAK-L3-006`: browser materialization diagnostics now include `browser_profile_compilation_v1`, making denied launch controls and separately admitted capabilities explicit.
- `CLOAK-L3-007`: broker diagnostics now emit `page_readiness_extraction_v1`, distinguishing evidence already packaged, blocker/shell pages, thin extraction, and materialization/fetch-needed states.
- `CLOAK-L3-008`: extraction readiness now records the main-text/metadata/raw-artifact contract without exposing raw bodies.
- `CLOAK-L3-011`: browser materialization diagnostics now include `browser_capability_readiness_lifecycle_v1`, keeping missing/install/update/cleanup state separate from search-result quality.
- `CLOAK-L3-013`: mock-fast tests cover allowed public URLs, internal/credentialed URL blocking, profile contract projection, and page-readiness projection.

Implementation files:

- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs`
- `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`

Mechanics now represented in diagnostics:

- Browser materialization only sees `candidate_url_ref_available` when at least one candidate passes the public HTTP/HTTPS URL safety screen.
- Internal hosts, URL credentials, and non-HTTP(S) locators block materialization recommendation and route back to alternate-provider/refinement behavior.
- Browser profile compilation is observable as a default-off Tool-CD contract, with raw browser args, debugging flags, certificate bypass, local file access, extensions, proxy/session fields, and raw scripts/CDP commands denied at the boundary.
- Page readiness/extraction status is separated from provider retrieval status so a provider can succeed while extraction remains thin, shell-blocked, or not yet promoted.
- Optional browser readiness is explicit and default-off: ordinary research does not install dependencies or launch a browser just because a result was low-signal.

Important boundary:

This wave still does not add live navigation, DNS/IP resolution, redirect traversal, DOM extraction, Playwright/Puppeteer invocation, proxy/session behavior, or page waiting. It makes those future mechanics explicit and testable at the diagnostic contract layer first.

## Mechanic 1: Provider Result Normalization

### Input Shapes

- Search result rows.
- Direct fetch responses.
- Provider errors.
- Partial provider failures.
- Future browser materialization output.
- Cached result metadata.

### Normalized Candidate Fields

- `source_kind`
- `source_class`
- `title`
- `locator`
- `source_domain`
- `snippet`
- `status_code`
- `timestamp`
- `permissions`
- `provider_id`
- `retrieval_stage`
- `artifact_ref`, if raw payload exists

### Normalized Error Fields

- `provider_id`
- `stage`
- `status_code`, if available
- `error_class`
- `retryable`
- `raw_error_ref`, if retained
- `safe_summary`

### Algorithm

```text
for each provider output:
  parse known success rows into candidates
  parse known error rows into normalized errors
  strip or quarantine raw payloads
  normalize status codes and provider names
  attach retrieval stage
  produce candidate list + error list
```

### Exit Criteria

- Tool diagnostics can explain which provider/stage failed without exposing raw provider payloads.
- Candidate rows from search, fetch, cache, and future browser materialization share enough fields to be scored together.

## Mechanic 2: Blocker Signal Extraction

### Signal Sources

- HTTP status codes: `401`, `403`, `407`, `408`, `409`, `425`, `429`, `451`, `5xx`.
- Provider errors: timeout, credential missing, provider disabled, rate limit, circuit open.
- Snippet/page markers: CAPTCHA, Cloudflare, bot wall, JavaScript required, access denied, login required, subscription required.
- Shell markers: empty page, app shell only, navigation chrome only, script-disabled message.
- Relevance markers: dictionary/shopping/forum rows when the user did not ask for that source class.

### Blocker Priority

| Priority | Class | Why |
| --- | --- | --- |
| 1 | `anti_bot_challenge` | Blocker pages must never become evidence. |
| 2 | `needs_js` | Static retrieval likely saw a shell, not content. |
| 3 | `rate_limited` | Retry/alternate provider may help. |
| 4 | `access_denied` | Usually needs alternate source or permission boundary. |
| 5 | `provider_degraded` | Tool health issue, not content quality. |
| 6 | `content_materialization_missing` | Candidate exists but content is too thin. |
| 7 | `off_intent_noise` | Search matched words but not task intent. |
| 8 | `low_signal` | Some signal exists but cannot support strong claims. |

### Algorithm

```text
derive blocker flags from provider errors
derive blocker flags from status codes
derive blocker flags from snippets and fetched text
derive off-intent flags from source class + query overlap + content type
dedupe flags
select primary blocker by priority
emit taxonomy rows with retryability and next-capability hints
```

### Exit Criteria

- A bad result can be classified as an access problem, rendering problem, provider problem, relevance problem, or evidence-quality problem.
- Anti-bot and access-denied material is quarantined before evidence promotion.

## Mechanic 3: Retrieval Decision Lattice

### Inputs

- User goal.
- Query/query pack submitted by the agent.
- Candidate count.
- Evidence count.
- Blocker taxonomy.
- Evidence coverage facets.
- Provider health.
- Remaining budgets.
- Capability admission state.

### Decisions

| Decision | Conditions |
| --- | --- |
| `synthesize_from_evidence` | Evidence is relevant, substantive, safe, and sufficient for the goal. |
| `agent_refine_query_pack` | Evidence is missing, off-intent, single-weak-source, or query terms are too broad. |
| `direct_fetch_candidate` | Search returns promising URLs with weak snippets. |
| `browser_materialize_candidate` | Candidate URL exists, content is shell/blocker/thin, browser capability is admitted, and safety passes. |
| `alternate_provider` | Provider degraded/rate-limited and another admitted provider exists. |
| `structured_low_evidence` | Budgets exhausted or capability unavailable, but the workflow must return useful uncertainty. |

### Algorithm

```text
if enough evidence:
  synthesize_from_evidence
else if blocker is anti_bot or needs_js and candidate URL exists:
  if browser materialization admitted and budget remains:
    browser_materialize_candidate
  else:
    agent_refine_query_pack or structured_low_evidence
else if blocker is anti_bot or needs_js and no candidate URL exists:
  alternate_provider or browser-capable retrieval if admitted
else if provider degraded or rate_limited:
  alternate_provider if available else agent_refine_query_pack
else if off_intent or weak_single_source:
  agent_refine_query_pack
else if promising candidates have thin snippets:
  direct_fetch_candidate or browser_materialize_candidate by policy
else:
  structured_low_evidence
```

### Exit Criteria

- The system can say why it chose retry/fetch/browser/synthesis without hardcoding a domain or prompt.
- Low-evidence output is reached by budgeted decision, not by silent failure.

## Mechanic 4: Query Refinement Signal Generation

### Purpose

Give the agent useful, domain-general signals for better query packs while preserving agent authority over the actual query text.

### Inputs

- Original user goal.
- Prior query/query pack.
- Candidate titles/snippets/domains.
- Missing coverage facets.
- Blocker taxonomy.
- Term hints.
- Source-class hints.

### Output Signals

- Preserve exact named entities when present.
- Split broad goals by entity, facet, timeframe, or source class.
- Prefer primary/official/research/current source classes when the goal requires evidence.
- Remove terms associated with off-intent rows.
- Add missing facets instead of replacing the whole user goal.
- Prefer another query pack before asking the user to narrow when budget remains.

### Non-Goals

- Do not generate hidden queries without agent submission.
- Do not hardcode specific research topics.
- Do not force output format.

### Exit Criteria

- Low-signal results give the agent enough diagnostic context to ask better questions of the web tooling.
- The tool provides strategy hints, not a scripted search plan.

## Mechanic 5: URL Safety Mechanics

### Algorithm

```text
parse URL
reject missing scheme
allow only http/https
reject username/password in URL
reject localhost, loopback, link-local, private, multicast, and metadata targets
resolve host when possible
enforce timeout and byte budget
navigate/fetch with redirect limit
validate each redirect target
validate final URL
redact unsafe details from diagnostics
```

### Exit Criteria

- Browser materialization cannot become local-file access, intranet probing, credential leakage, or redirect-based SSRF.

## Mechanic 6: Browser Profile Compilation

### Inputs

- Tool CD profile policy.
- Capability admission.
- Runtime provider metadata.
- Request-level allowed options.

### Denied Caller Controls

- Raw browser args.
- Raw launch args.
- Remote debugging flags.
- Certificate bypass flags.
- Local file access flags.
- Extension load flags.
- Proxy/session fields without separate capability.
- Raw scripts/CDP commands.

### Algorithm

```text
start with Tool CD default profile
merge admitted runtime profile
merge allowed request fields
reject denied fields
normalize duplicate/conflicting settings
produce effective profile summary + profile hash
keep raw launch details telemetry-only
```

### Exit Criteria

- Future browser adapters get one deterministic profile object.
- Callers cannot smuggle browser authority through request fields.

## Mechanic 7: Page Settle / Readiness

### Candidate Readiness Signals

- DOM content loaded.
- Network idle or bounded quiet window.
- Main-text length threshold.
- Query-term overlap appears in visible text.
- Known blocker markers absent.
- Selector requested by admitted policy appears.
- Redirect chain stabilized.

### Algorithm

```text
wait for primary readiness event within timeout
sample visible/main text
if blocker markers appear:
  classify blocker and stop
if content is still shell-only:
  optionally wait one bounded cycle
if substantive text appears:
  proceed to extraction
else:
  classify content_materialization_missing
```

### Exit Criteria

- Browser materialization stops because it is ready, blocked, too thin, or timed out.
- It does not wait indefinitely or pretend shell text is evidence.

## Mechanic 8: Main Content Extraction

### Extraction Order

1. Metadata: title, canonical URL, status, content type.
2. Main text/readability extraction if available.
3. Fallback visible text extraction.
4. Link summary.
5. Blocker marker scan.
6. Bounded artifact ref for raw payload, if retained.

### Output Constraints

- Main text is bounded.
- Raw HTML is artifact-only.
- Console/network logs are artifact-only.
- Cookies/storage/proxy details are not retained in evidence.

### Exit Criteria

- Extraction output can become a normal evidence candidate after scoring.
- Failed extraction produces a blocker/quality class instead of blank success.

## Mechanic 9: Evidence Promotion Scoring

### Scoring Components

- Query/entity overlap.
- Source trust/source class.
- Content richness.
- Claim-hint availability.
- Freshness signal when relevant.
- Source diversity.
- Blocker absence.
- Extraction confidence.
- Permission/publicness.

### Promotion Rules

```text
if unsafe URL or denied payload:
  reject
if blocker text:
  reject
if no substantive main text:
  low-confidence retained or reject
if no query relevance:
  low-confidence retained or reject
if enough relevance + substance:
  promote to evidence pack
attach source class, confidence, quality flags, coverage facets
```

### Exit Criteria

- Browser-enriched content and normal search/fetch content pass through the same evidence-quality discipline.
- `browser_enriched` is a marker, not a trust override.

## Mechanic 10: Retry Budget And Stop Conditions

### Budgets

- Query pack attempts.
- Provider attempts.
- Direct fetch attempts.
- Browser materialization attempts.
- Per-domain attempt caps.
- Total elapsed time.
- Total result count.
- Evidence pack size.

### Stop Conditions

- Evidence is sufficient for the goal.
- Same failure signature repeats.
- Same domain keeps producing blockers.
- Budget exhausted.
- Capability unavailable.
- Safety block encountered.

### Exit Criteria

- The system tries harder when the next attempt is likely to improve evidence.
- It stops cleanly when it would only churn.

## Mechanic 11: Readiness Lifecycle

### States

- `not_configured`
- `not_installed`
- `version_mismatch`
- `ready`
- `degraded`
- `blocked`
- `cleanup_required`

### Algorithm

```text
read admitted provider config
check dependency presence if configured
check version constraints if available
check runtime health probe if cheap/safe
check cleanup budget and artifact size
emit status without installing during ordinary research
```

### Exit Criteria

- Optional browser capability is measurable but never surprise-installed by normal research.

## Mechanic 12: Artifact Quarantine

### Artifact Classes

- Raw fetched body.
- Raw rendered HTML.
- Screenshot, if enabled.
- Redirect summary.
- Provider error body.
- Browser logs, if retained.

### Algorithm

```text
store raw payload behind artifact ref
attach retention and cleanup metadata
project only safe summaries into diagnostics
allow evidence pack only after extraction and scoring
never inject artifact body into final chat
```

### Exit Criteria

- Debugging remains possible without leaking raw operational payloads into synthesis or chat.

## Level 3 Test Plan

| Test ID | Purpose | Fixture Shape |
| --- | --- | --- |
| `CLOAK-L3-T001` | Provider normalization keeps success/error rows comparable. | Mixed search/fetch/provider error fixture. |
| `CLOAK-L3-T002` | Anti-bot and JS shells are blockers, not evidence. | CAPTCHA and JS-required snippets. |
| `CLOAK-L3-T003` | Retry lattice selects query refinement for off-intent low signal. | Dictionary/shopping/forum mismatch fixture. |
| `CLOAK-L3-T004` | Retry lattice selects direct fetch for promising thin snippets. | URL-rich, snippet-thin fixture. |
| `CLOAK-L3-T005` | Browser materialization is recommended only when admitted and safe. | Blocker + candidate URL + admission matrix. |
| `CLOAK-L3-T006` | URL safety rejects unsafe schemes and private redirects. | `file:`, localhost, private IP, redirect fixture. |
| `CLOAK-L3-T007` | Profile compiler rejects denied caller controls. | Request with browser args/proxy/session/script fields. |
| `CLOAK-L3-T008` | Page settle classifies shell-only output as materialization missing. | Rendered shell text fixture. |
| `CLOAK-L3-T009` | Evidence promotion marks browser-enriched but does not trust it by default. | Browser output with relevant main text. |
| `CLOAK-L3-T010` | Repeated same failure signature stops retry churn. | Repeated blocker/error fixture. |

## Implementation Candidate Order

1. Strengthen provider result normalization and blocker signal extraction in diagnostics.
2. Add a retrieval decision lattice artifact to the quality report.
3. Add query-refinement signal fields that remain agent-authoritative.
4. Add direct-fetch versus browser-materialization recommendation split.
5. Add URL safety mock tests before any live browser executor.
6. Add profile compiler contract tests before any adapter launch code.
7. Add page-settle/extraction fixture tests.
8. Add evidence-promotion fixture tests for browser-enriched candidates.

## Level 3 Exit Criteria

Before moving to Level 4 implementation-structure mapping, we needed:

- A concrete decision lattice that explains retry/fetch/browser/synthesis/low-evidence outcomes.
- Blocker extraction rules for provider errors, status codes, and text markers.
- Query-refinement signals that help the agent without hidden query generation.
- URL safety mechanics written as algorithm and fixture expectations.
- Browser profile compilation mechanics written as algorithm and fixture expectations.
- Page settle and extraction mechanics written as algorithm and fixture expectations.
- Evidence promotion mechanics that treat browser-enriched content as candidate evidence only.
- A test list that can be mapped directly to files/modules in Level 4.

## Current Assessment

The highest-ROI Level 3 work is not the live browser executor yet. It is the decision lattice and evidence-promotion mechanics. Those are the pieces that tell us whether weak web results should trigger another query, direct fetch, browser materialization, alternate provider, or a bounded low-evidence answer.

Once that decision lattice is explicit and tested, a future browser executor can plug into a known lane instead of becoming another black box.

Level 4 owner mapping now lives in `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md`.
