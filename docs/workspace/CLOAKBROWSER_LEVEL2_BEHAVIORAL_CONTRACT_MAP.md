# CloakBrowser Level 2 Behavioral Contract Map

Created: 2026-05-13

Source assimilation ledger: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Purpose

This document is the next level down from high-level pattern assimilation. It maps CloakBrowser-inspired patterns into behavioral contracts for Infring web tooling before we move into mechanics or syntax-level implementation.

The goal is to define what each primitive must accept, produce, classify, reject, hide, expose, and prove.

This is not a source-copy plan. It is a contract map for portable behavior.

## Level Stack

| Level | Name | Question Answered | Current Artifact |
| --- | --- | --- | --- |
| 1 | Architecture / pattern | What patterns are useful and where do they belong? | CloakBrowser assimilation ledger |
| 2 | Behavioral contract | What must the system behaviorally accept, return, classify, and prove? | This document |
| 3 | Mechanics / algorithm | How should classification, retry, settling, extraction, and scoring work? | Next wave |
| 4 | Implementation structure | Which modules, files, CD fields, adapters, and tests own each behavior? | Later wave |
| 5 | Syntax | Exact code, branches, field access, regexes, and assertions. | Last wave |

## Guardrails

- Browser materialization is an optional recovery capability, not default web search.
- Workflow CDs should consume capability outcomes, not own browser behavior.
- Tool CDs declare capability shape, inputs, outputs, budgets, side effects, safety boundaries, and evidence policy.
- Runtime code may execute and validate primitive behavior, but must not invent research-specific workflow behavior.
- Browser, proxy, persistent sessions, humanized interaction, and service pools must remain separately admitted capabilities.
- Raw browser traces, proxy details, cookies, page internals, console logs, and network payloads are telemetry-only unless later synthesized at a high level.
- Contracts must stay cross-domain. They cannot assume software research, benchmark prompts, Infring comparisons, or any particular output format.

## System Destination Map

| Behavioral Contract | CloakBrowser Source Pattern | Infring Destination | Priority | Status |
| --- | --- | --- | --- | --- |
| Capability admission | Browser launch wrappers and service wrapper | Tool CD + web-conduit provider registry | high | integrated |
| Request envelope | Launch/context wrappers, Lambda handler | Tool CD request schema + Gateway/web-conduit validation | high | integrated at contract/catalog layer |
| URL and redirect safety | Lambda security tests | Gateway/web-conduit safety validation | high | contract integrated |
| Launch/profile normalization | `args.ts`, config, context option filtering | Tool CD profile schema + adapter normalization | high | contract integrated |
| Runtime readiness | Binary/cache/update surfaces | web-conduit status and provider readiness | high | integrated |
| Blocker classification | Anti-detection tests and browser failure classes | web tooling diagnostics + retrieval gates | high | integrated |
| Materialized page output | Browser fetch/materialization wrappers | evidence-pack candidate enrichment lane | high | contract integrated |
| Evidence handoff | Extracted page content | batch query evidence pack + synthesis input | high | integrated at contract layer |
| Proxy secret separation | Proxy parser/tests | Gateway secret broker + optional proxy Tool CD | medium | not integrated |
| Geo/proxy consistency | GeoIP modules | browser profile metadata | medium | deferred |
| Humanized interaction | `human/**` | dynamic page interaction capability | medium | deferred |
| Persistent session lifecycle | persistent context APIs | session capability handles + cleanup | medium | deferred |
| Browser service pool | `cloakserve` | optional browser provider service | low/medium | deferred |
| Mock-fast contract tests | unit tests around launch/proxy/stealth | ops/web tooling tests | high | integrated |

## Contract 1: Browser Materialization Capability Admission

### Purpose

Declare a browser-backed page materialization lane that can be used when ordinary search/fetch returns blocked, dynamic, or too-thin content and policy admits browser execution.

### Inputs

- Capability id.
- Requested URL or candidate URL ref.
- Evidence gap or blocker reason.
- Policy profile name.
- Budget envelope.
- Admission state.

### Outputs

- Capability availability state.
- Selected provider id.
- Execution gate: allowed, blocked, unavailable, degraded, or not enabled.
- Blocking reason.
- Retry recommendation.
- Telemetry-only diagnostics.

### States

| State | Meaning |
| --- | --- |
| `capability_not_enabled` | Browser materialization is intentionally default-off. |
| `adapter_not_ready` | Policy allows the capability, but no admitted adapter is ready. |
| `permission_required` | The capability requires a permission boundary not currently satisfied. |
| `ready` | The provider is admitted and can be invoked. |
| `blocked_by_policy` | The request or context violates policy. |
| `runtime_failed` | The provider was admitted but failed at execution time. |

### System Mapping

- Tool CD: `web_retrieval_v0.tool.json`
- Runtime status: web-conduit provider runtime metadata
- Effective inventory: web tooling inventory and policy snapshot
- Diagnostics: batch-query web quality report

### Acceptance Tests

- Default policy shows browser materialization as optional and disabled.
- Disabled optional capability does not count as a blocked core web tool.
- Enabled-but-not-ready state is degraded, not silently ignored.
- Tool catalog exposes capability metadata without exposing execution authority to workflow CDs.

## Contract 2: Browser Materialization Request Envelope

### Purpose

Provide one normalized request shape for browser page materialization, regardless of whether the future implementation uses a local Playwright adapter, service pool, or another admitted adapter.

### Required Inputs

- `url`
- `request_id` or run context ref
- `budget`
- `admission_ref`

### Optional Inputs

- `extract_mode`
- `wait_until`
- `wait_for_selector`
- `timeout_ms`
- `max_response_bytes`
- `profile_ref`
- `evidence_gap_reason`

### Denied Inputs

- Raw browser args from caller.
- Proxy URL or credentials unless proxy capability is separately admitted.
- Persistent session identifiers unless session capability is separately admitted.
- Arbitrary user scripts.
- Raw CDP commands.
- Local file paths.

### System Mapping

- Tool CD request schema.
- Gateway/web-conduit input validation.
- Browser adapter normalization layer.

### Acceptance Tests

- Missing URL fails closed.
- Non-http schemes are rejected.
- Caller-provided browser args are rejected.
- Proxy/session fields fail closed unless separately admitted.
- Request is normalized before adapter execution.

## Contract 3: URL And Redirect Safety

### Purpose

Prevent browser materialization from becoming an SSRF, local-file, intranet, credential, or redirect bypass primitive.

### Required Safety Checks

- Allow only `http` and `https`.
- Reject localhost, loopback, link-local, private network, and metadata service targets.
- Resolve and validate host before navigation when possible.
- Revalidate final URL after redirects.
- Bound redirect count.
- Bound response bytes.
- Bound navigation timeout.
- Strip credentials from projected URL fields.

### Outputs

- `url_safety_status`
- `source_url`
- `final_url`
- `redirect_chain_ref` or bounded redirect summary
- `blocked_reason`

### System Mapping

- Gateway/web-conduit safety validator.
- Tool CD security contract.
- Telemetry-only blocker diagnostics.

### Acceptance Tests

- `file://`, `ftp://`, `data:`, localhost, and private IPs are rejected.
- Public URL redirecting to private target is rejected after redirect validation.
- URL credentials are redacted from diagnostics.
- Final answer never receives raw unsafe URL traces.

## Contract 4: Launch/Profile Normalization

### Purpose

Centralize browser launch/profile settings so provider behavior is auditable, deterministic, and not scattered through workflow or adapter code.

### Inputs

- Profile id.
- Headless mode policy.
- Viewport.
- Locale and timezone.
- User agent profile, if admitted.
- Storage/session scope, if admitted.
- Resource policy.
- Safe default args.
- Denied args.

### Outputs

- Compiled launch profile artifact.
- Denied option list.
- Effective profile summary.
- Profile hash or fingerprint.

### Invariants

- Profile compilation happens before adapter launch.
- Caller cannot override denied flags.
- Context-level options that conflict with profile-level settings are normalized or rejected.
- Unsafe debugging, certificate bypass, local file access, and arbitrary extension flags are denied unless an explicit capability admits them.

### System Mapping

- Tool CD browser profile schema.
- web-conduit adapter profile compiler.
- Provider readiness diagnostics.

### Acceptance Tests

- Duplicate/conflicting settings produce one deterministic effective profile.
- Denied flags are removed or rejected.
- Locale/timezone settings have one owner.
- Effective profile is telemetry-visible but raw launch args are not chat-visible.

## Contract 5: Runtime Readiness Lifecycle

### Purpose

Keep heavyweight optional browser dependencies measurable without surprise installation or hidden runtime behavior.

### States

| State | Meaning |
| --- | --- |
| `not_configured` | Capability exists but no provider is configured. |
| `not_installed` | Provider dependency is missing. |
| `version_mismatch` | Installed dependency does not match admitted version constraints. |
| `ready` | Provider can run. |
| `degraded` | Provider can partially run but has warnings. |
| `blocked` | Policy or environment blocks provider. |
| `cleanup_required` | Provider state exceeds lifecycle limits. |

### Outputs

- Provider id.
- Version, if safe to reveal.
- Readiness status.
- Cleanup status.
- Installation status.
- Diagnostic refs.

### System Mapping

- web-conduit provider runtime metadata.
- Tooling inventory.
- Optional future dependency manager.

### Acceptance Tests

- Missing provider is reported as unavailable, not silently ignored.
- Default runs do not download browser binaries.
- Cleanup state is bounded and attached to system cleanup lifecycle.

## Contract 6: Blocker Classification

### Purpose

Separate bad retrieval quality from access blockers so we know whether to improve search, retry, fetch directly, materialize in browser, or fail with low evidence.

### Blocker Classes

| Class | Meaning |
| --- | --- |
| `anti_bot_challenge` | CAPTCHA, challenge page, human verification, bot wall. |
| `needs_js` | Static fetch/search sees a shell that requires JavaScript. |
| `rate_limited` | HTTP or provider throttling. |
| `access_denied` | Forbidden, login wall, region block, subscription wall. |
| `provider_degraded` | Tool/provider readiness or credential issue. |
| `content_materialization_missing` | Candidate exists but usable content is too thin. |
| `off_intent_noise` | Result is unrelated despite lexical overlap. |
| `low_signal` | Result exists but is too weak to support claims. |

### Outputs

- `blocker_class`
- `retryable`
- `recommended_next_capability`
- `evidence_impact`
- `telemetry_summary`

### System Mapping

- Batch-query web quality diagnostics.
- Web retrieval gates.
- Evidence pack quality.
- Browser materialization recovery report.

### Acceptance Tests

- Anti-bot snippets do not become evidence.
- Access-denied snippets do not become evidence.
- Off-intent dictionary/shopping/lyrics style results do not become evidence unless the user asked for that class.
- Browser materialization is recommended only as telemetry/policy guidance when blockers match and capability is admitted.

## Contract 7: Materialized Page Output

### Purpose

Return page content in a form the evidence pack can consume without leaking raw browser internals.

### Required Outputs

- `source_url`
- `final_url`
- `status_code`
- `title`
- `main_text_or_markdown`
- `links_summary`
- `blocker_classification`
- `extraction_confidence`
- `artifact_ref`

### Optional Outputs

- `content_type`
- `document_type`
- `readability_score`
- `render_wait_summary`
- `redirect_summary`
- `screenshot_ref`, if separately enabled

### Denied Outputs To Chat

- Raw HTML.
- Raw console logs.
- Raw network logs.
- Cookies.
- Storage state.
- Proxy details.
- CDP traces.

### System Mapping

- Browser adapter output contract.
- Evidence-pack candidate enrichment lane.
- Artifact store for raw/quarantined payload refs.

### Acceptance Tests

- Output can produce an evidence pack item.
- Raw page/browser payload is quarantined.
- Empty or shell-only pages are classified instead of promoted.
- Extraction confidence flows into evidence quality.

## Contract 8: Evidence Pack Handoff

### Purpose

Ensure browser materialization improves synthesis only after extracted content is converted into evidence-pack form.

### Inputs

- Materialized page output.
- Original query.
- Required coverage metadata.
- Current evidence gaps.
- Source classification policy.

### Outputs

- Evidence item.
- Claim hints.
- Term hints.
- Source class.
- Confidence.
- Coverage facets.
- Quality flags.

### Invariants

- Browser success is not source truth by itself.
- Extracted content must pass the same relevance and quality filters as normal web candidates.
- Evidence pack can mark browser-enriched content without making it chat-visible raw payload.

### System Mapping

- Batch-query evidence pack.
- retrieval broker diagnostics.
- synthesis input.

### Acceptance Tests

- Browser materialized content with no query overlap is rejected or low-confidence retained.
- Browser materialized content with blocker text is not evidence.
- Browser materialized content with substantive query-relevant text becomes evidence with a browser-enriched marker.

## Contract 9: Proxy Secret Separation

### Purpose

Keep proxy behavior available as a future capability without making it ambient or leaking secrets.

### Inputs

- Proxy capability ref.
- Secret handle.
- Proxy type.
- Bypass policy.
- Admission state.

### Outputs

- Redacted proxy summary.
- Proxy capability status.
- Proxy error class.

### Invariants

- Raw proxy URLs and credentials are never stored in final answers or evidence packs.
- Proxy fields on browser materialization requests are rejected unless proxy capability is admitted.
- Proxy use is not a default recovery strategy.

### System Mapping

- Gateway secret broker.
- Optional proxy Tool CD.
- Browser profile compiler.

### Acceptance Tests

- Proxy URL credentials are redacted.
- SOCKS/http distinctions are normalized internally.
- Proxy fields fail closed without capability admission.

## Contract 10: Humanized Interaction Primitive

### Purpose

Treat scroll, click, type, and wait actions as explicit dynamic page interaction primitives with budgets, not hidden behavior inside search.

### Inputs

- Action type.
- Target selector or target coordinates from trusted page inspection.
- Timing budget.
- Interaction profile ref.
- Page/session handle.

### Outputs

- Action receipt.
- Resulting page state summary.
- Blocker or completion classification.

### Invariants

- No interaction runs by default.
- Interaction steps are budgeted and separately admitted.
- User-supplied scripts are not executed.
- Raw page internals stay telemetry-only.

### System Mapping

- Future dynamic page interaction Tool CD.
- Browser materialization adapter internals.

### Acceptance Tests

- Interaction request without active admitted session fails closed.
- Time budget caps action sequences.
- DOM-read output is bounded and sanitized.

## Contract 11: Persistent Session Lifecycle

### Purpose

Allow future session reuse only when explicitly admitted and bounded.

### Inputs

- Session capability handle.
- TTL.
- Scope.
- Cleanup policy.
- Identity/retention class.

### Outputs

- Session handle ref.
- Expiry state.
- Cleanup receipt.
- Active refcount.

### Invariants

- Default state is stateless.
- No implicit persistent profile is created by browser materialization.
- Active sessions are not cleaned while refcounted.
- Expired sessions are tied to system cleanup.

### System Mapping

- Future browser session manager.
- Runtime cleanup lifecycle.
- Tool CD session policy.

### Acceptance Tests

- Stateless request does not create persistent state.
- Expired session is cleanup-eligible.
- Active refcount blocks cleanup.

## Contract 12: Browser Service Pool

### Purpose

Capture the CloakBrowser service-pool pattern as a future performance optimization without exposing raw CDP/debugging authority.

### Inputs

- Provider service handle.
- Session or seed ref.
- Capability admission.
- Request budget.

### Outputs

- Connection lease ref.
- Provider status.
- Refcount.
- Cleanup/close receipt.

### Invariants

- Workflows never receive raw remote debugging URLs.
- Service pool is optional and below the web-conduit provider boundary.
- Per-request artifacts still pass through evidence-pack conversion.

### System Mapping

- Future browser provider service.
- Gateway/web-conduit adapter.
- Runtime provider readiness.

### Acceptance Tests

- Lease creation and release are refcounted.
- Raw debugging endpoint is not projected.
- Closed/expired sessions cannot be reused.

## Cross-Cutting Visibility Rules

| Data | Chat Visible | Evidence Visible | Telemetry Visible |
| --- | --- | --- | --- |
| Final synthesized answer | yes | n/a | yes |
| Source URL/final URL | synthesized only | yes | yes |
| Main text excerpt | synthesized only | yes | yes |
| Blocker class | synthesized at high level | yes | yes |
| Raw HTML | no | no | ref only |
| Raw browser trace | no | no | ref only |
| Console/network logs | no | no | ref only |
| Cookies/storage | no | no | no, except redacted state refs |
| Proxy credentials | no | no | no |
| Profile hash/status | no | no | yes |
| Capability state | no, unless user asks status | no | yes |

## Level 2 Assimilation Backlog

| ID | Contract | Next Action | Target |
| --- | --- | --- | --- |
| `CLOAK-L2-001` | Launch/profile normalization | Add Tool CD profile schema and denied-field contract. | Tool CD |
| `CLOAK-L2-002` | Request envelope | Tighten browser materialization request schema with allowed/denied fields. | Tool CD + web-conduit contract |
| `CLOAK-L2-003` | URL/redirect safety | Add mock safety tests for unsafe schemes and redirects. | Web tooling tests |
| `CLOAK-L2-004` | Blocker classification | Add blocker taxonomy fields to quality report and gates. | batch-query diagnostics |
| `CLOAK-L2-005` | Materialized page output | Define evidence-pack conversion contract for browser-enriched candidates. | Tool CD + batch-query |
| `CLOAK-L2-006` | Runtime readiness | Add readiness lifecycle fields for installed/version/cleanup states. | web-conduit status |
| `CLOAK-L2-007` | Proxy separation | Draft proxy capability contract without enabling it. | Tool CD/Gateway |
| `CLOAK-L2-008` | Mock-fast tests | Add tests for request denial, blocker classification, and evidence handoff. | ops tests |
| `CLOAK-L2-009` | Service pool | Document future pool interface but defer implementation. | Future provider service |

## Level 2 Execution: Behavioral Contracts

Status: integrated for the non-executor contract layer.

Implemented:

- Added a browser materialization request contract with required, optional, and denied fields in the web conduit policy/default policy and the web retrieval Tool CD.
- Added a browser profile contract with stateless default scope, denied launch flags, no caller launch-arg override, and telemetry-only profile summaries.
- Added security contract fields for URL credential rejection, redirect bounds, and safe/final URL status vocabulary.
- Added a blocker taxonomy in web quality diagnostics that separates anti-bot challenge, JavaScript-required shells, rate limiting, access denied, provider degradation, missing materialized content, off-intent noise, and low signal.
- Added an evidence handoff contract that keeps browser materialization in a candidate-enrichment lane and requires safe URL, substantive main text, query relevance, and non-blocker content before evidence promotion.
- Added readiness lifecycle fields so browser materialization remains default-off, measurable, cleanup-bound, and not surprise-installed during ordinary research.
- Added mock-fast tests for default-off catalog/status projection, anti-bot blocker recovery, blocker taxonomy splitting, and evidence-handoff visibility.

Validation:

- `jq empty core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `cargo test -p infring-ops-core status_bootstraps_default_policy_and_receipts_surface -- --nocapture`
- `cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`
- `git diff --check` on the Level 2 touched files

Boundary:

This stage still does not add a live browser executor, persistent sessions, proxy behavior, humanized interaction, or a service pool. It makes those future mechanics admissible and measurable without making browser behavior an ambient web-search default.

## Exit Criteria For Level 2

Before moving to mechanics/algorithm level, the system should have:

- A declared request contract.
- A declared output contract.
- A declared safety contract.
- A declared blocker taxonomy.
- A declared evidence-pack handoff contract.
- A declared readiness lifecycle.
- Mock-fast tests proving default-off behavior and failure classification.
- Ledger entries showing which contracts are integrated, partial, queued, or deferred.

## Current Assessment

The next implementation wave should focus on `CLOAK-L2-001` through `CLOAK-L2-005`. That would make the browser-materialization capability much less black-boxy without adding a live browser executor yet.

The most important behavioral line to preserve is:

```text
browser materialization may improve candidate enrichment after admission,
but only evidence-pack output may feed synthesis.
```
