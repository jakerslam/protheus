# CloakBrowser Level 5 Syntax Implementation Map

Created: 2026-05-14

Source assimilation ledger: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Level 4 implementation map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md`

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Purpose

Level 5 is the syntax-level assimilation map. It is where we stop treating CloakBrowser as a source of broad patterns and start working through source files one at a time to extract concrete implementation shapes.

This does not mean copying CloakBrowser code into Infring. It means reading each relevant file closely enough to decide:

- which exact control flow should become an Infring primitive,
- which syntax/API details are specific to CloakBrowser and should stay out,
- which parts must remain deferred behind capability admission,
- which tests prove the behavior without requiring live anti-bot sites,
- which Infring files should change when we implement the extracted pattern.

## Level 5 Rule

No Level 5 item is considered assimilated until it has all five statuses:

```text
parsed -> pattern extracted -> Infring target mapped -> integrated or rejected -> tested or deferred with reason
```

Previous levels parsed enough of many files to design the architecture. Level 5 resets the bar: each file below needs its own focused syntax pass before we claim full assimilation.

## Guardrails

- Keep browser materialization default-off and capability-admitted.
- Do not add stealth browser behavior to ordinary search by default.
- Do not add proxy, persistent session, humanized interaction, raw CDP, or user-script behavior as implied subfeatures.
- Do not allow caller-supplied browser args, launch args, CDP commands, user scripts, proxy fields, session IDs, storage state, or local file targets through the materialization request.
- Do not expose raw HTML, screenshot bytes, console logs, network traces, browser launch args, or CDP traces to chat.
- Prefer fixture/mock-fast tests before live browser probes.
- Keep workflow CDs provider-agnostic. Browser syntax belongs in web-conduit/provider code, not in research workflow Rust or final-answer prompts.

## Current Infring Landing Zone

The Level 4 slices created the boundary that Level 5 should fill:

| Surface | Current State | Level 5 Role |
| --- | --- | --- |
| Tool CD | `web_retrieval_v0.tool.json` declares `browser_materialize_page`. | Extend only when syntax pass proves a missing field is necessary. |
| Policy | `browser_materialization` is default-off with request/profile/security/output/evidence contracts. | Add adapter-specific readiness fields only after syntax pass. |
| API | `api_browser_materialize_page` validates URL/admission, denied fields, readiness, and handoff contract. | Add fake/live provider execution behind this boundary. |
| CLI | `web-conduit browser-materialize` routes to the same API boundary. | Use as local proof harness; do not create a second path. |
| Tests | Browser materialization contract tests pass mock-fast. | Add syntax-level adapter tests before live probes. |
| Evidence | Output and handoff contracts are projected, but no materialized page exists yet. | Convert page output to evidence candidate and artifact refs. |

## Assimilation Strategy

Level 5 should proceed in slices, each grounded in specific source files:

1. One-shot materialization security and extraction loop.
2. Launch/profile argument construction.
3. Playwright/Puppeteer adapter parity.
4. Readiness and dependency lifecycle.
5. Proxy/geo/session/humanization deferred capability maps.
6. Service/pool architecture only if single-shot adapter is stable.

Do not jump to proxy/session/humanized behavior before the basic one-shot materialization loop is implemented and measured.

## File-by-File Burn-Down

Status values:

- `pending`: not yet reviewed at Level 5 depth.
- `in_progress`: current focused file.
- `mapped`: syntax-level pattern extracted and mapped, not integrated.
- `integrated`: pattern implemented in Infring.
- `rejected`: intentionally not assimilated.
- `deferred`: useful but not for the current primitive.

| Order | Source File | Level 5 Status | Primary Question | Likely Infring Target |
| ---: | --- | --- | --- | --- |
| 1 | `examples/integrations/aws_lambda/lambda_handler.py` | mapped seed, needs line pass | What is the smallest safe one-shot navigate/wait/capture/close loop? | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` plus future adapter part. |
| 2 | `tests/test_lambda_security.py` | mapped seed, needs line pass | Which security invariants must be locked before live browser execution? | Browser materialization contract tests. |
| 3 | `js/src/playwright.ts` | mapped seed, needs line pass | What launch/context cleanup and option filtering shape should the adapter mimic? | Future local browser adapter helper. |
| 4 | `js/src/types.ts` | pending | Which request/profile fields are real API surface versus convenience wrappers? | Tool CD/policy schema audit. |
| 5 | `js/src/args.ts` | mapped seed, needs line pass | How should profile args be deduped and overridden without caller authority? | Profile compiler tests and denied-field projection. |
| 6 | `js/src/config.ts` | pending | Which defaults are portable, and which are CloakBrowser-specific stealth baggage? | Provider readiness/config projection. |
| 7 | `js/src/download.ts` | pending | What dependency lifecycle patterns are useful without surprise installs? | Optional readiness/install plan, deferred. |
| 8 | `js/src/proxy.ts` | mapped seed, deferred | Which parsing/redaction rules are worth keeping if proxy capability is admitted later? | Gateway secret/proxy capability, deferred. |
| 9 | `js/src/geoip.ts` | pending, deferred | Which geo consistency fields belong in telemetry versus request authority? | Proxy/geo capability, deferred. |
| 10 | `js/src/puppeteer.ts` | pending | What adapter parity constraints matter if multiple browser runtimes are admitted? | Cross-adapter contract tests. |
| 11 | `cloakbrowser/browser.py` | mapped seed, needs line pass | What Python wrapper semantics confirm the JS adapter pattern? | Cross-runtime parity notes, not direct Rust code. |
| 12 | `cloakbrowser/config.py` | pending | Which config defaults map to policy and which should be rejected? | Policy/default profile compiler. |
| 13 | `cloakbrowser/download.py` | pending | What cache/version/checksum lifecycle is useful for optional providers? | Dependency readiness lifecycle, deferred. |
| 14 | `cloakbrowser/geoip.py` | pending, deferred | What proxy-exit metadata is useful but permission-sensitive? | Proxy/geo capability, deferred. |
| 15 | `bin/cloakserve` | mapped seed, deferred | Is CDP service pooling worth a future provider mode? | Service/pool capability, deferred. |
| 16 | `tests/test_cloakserve.py` | mapped seed, deferred | Which pool/session invariants would be required before service mode? | Future service/pool tests. |
| 17 | `tests/test_launch.py` | pending | What launch contract tests can be ported without live detection sites? | Browser adapter mock-fast tests. |
| 18 | `tests/test_launch_context.py` | pending | Which context cleanup and option filtering cases matter? | Browser adapter context tests. |
| 19 | `tests/test_build_args.py` | pending | Which arg compiler invariants must be copied as tests, not code? | Profile compiler tests. |
| 20 | `tests/test_backend.py` | pending | Is backend selection relevant to Infring or CloakBrowser-only? | Probably reject/defer. |
| 21 | `tests/test_config.py` | pending | Which config/default invariants are portable? | Policy/default tests. |
| 22 | `tests/test_proxy.py` | pending, deferred | Which proxy parsing and redaction tests belong to a future admitted proxy lane? | Proxy capability tests, deferred. |
| 23 | `tests/test_geoip.py` | pending, deferred | Which geo tests belong behind proxy admission? | Geo/proxy capability tests, deferred. |
| 24 | `tests/test_update.py` | pending, deferred | Which update tests inform provider dependency lifecycle? | Optional readiness/install tests. |
| 25 | `tests/test_extract.py` | mapped seed, deferred | Which archive hardening rules matter if provider install/extract exists? | Dependency extraction guard, deferred. |
| 26 | `js/src/human/config.ts` | mapped seed, deferred | What action-budget schema would be needed later? | Human interaction capability, deferred. |
| 27 | `js/src/human/index.ts` | mapped seed, deferred | Which isolated-world DOM-read pattern is useful for extraction without interaction? | Possible read-only DOM extraction, later. |
| 28 | `js/src/human/mouse.ts` | pending, deferred | Which movement primitives are out of scope for research retrieval? | Human interaction capability, deferred. |
| 29 | `js/src/human/keyboard.ts` | pending, deferred | Which trusted-input primitives are out of scope for research retrieval? | Human interaction capability, deferred. |
| 30 | `js/src/human/scroll.ts` | pending, deferred | Which scroll/readiness ideas can be reduced to read-only page settling? | Page readiness, later. |
| 31 | `tests/test_stealth_unit.py` | mapped seed, deferred | Which isolated-world tests can become read-only extraction tests? | Browser extraction tests, later. |
| 32 | `tests/test_humanize_unit.py` | pending, deferred | Which interaction tests should remain out of the first adapter? | Human interaction capability, deferred. |
| 33 | `tests/test_persistent_context.py` | pending, deferred | Which session-retention invariants are required before persistent profiles? | Session capability, deferred. |
| 34 | `examples/integrations/browser_use_example.py` | pending, reference only | Does integration style reveal useful adapter ergonomics? | Usually reject for core primitive. |
| 35 | `examples/integrations/crawl4ai_example.py` | pending, reference only | Does integration style reveal useful page extraction handoff? | Compare with existing Crawl4AI assimilation. |
| 36 | `examples/integrations/scrapling_example.py` | pending, reference only | Does integration style reveal useful fetch fallback handoff? | Compare with Scrapling assimilation. |

## First Slice: One-Shot Materialization

The first Level 5 implementation target should be a single-shot materialization adapter skeleton. It should be built from the Lambda handler pattern, not from the full stealth-browser wrapper.

Required syntax patterns to extract:

| Pattern | CloakBrowser Source | Infring Rule |
| --- | --- | --- |
| Validate URL before navigation | `lambda_handler.py::_validate_url` | Already partly integrated through fetch SSRF guard; live adapter must call before launch/navigation. |
| Navigate once with bounded timeout | `lambda_handler.py::_attempt_scrape` | Adapter must take timeout from policy/request bounds, not caller freeform. |
| Revalidate final URL after navigation | `lambda_handler.py::_attempt_scrape` | Must run before content extraction or artifact creation. |
| Smart DOM settle wait | `lambda_handler.py::_smart_wait` | Convert to policy-owned wait strategy; do not expose raw JS to caller. |
| Optional selector wait | `lambda_handler.py::_post_nav_waits` | Allowed only as request field already declared; must be bounded. |
| Capture title, final URL, HTML | `lambda_handler.py::_attempt_scrape` | Store raw HTML by artifact ref; expose title/final URL and extracted text only through contract. |
| Retry classification | `lambda_handler.py::_classify_error` | Start as telemetry/retry recommendation; do not add hidden retry loops before budgets are explicit. |
| Always close context | `lambda_handler.py::_attempt_scrape` | Adapter must close browser/context on every path and emit cleanup status. |

First slice exit criteria:

- A fake provider can return a deterministic materialized page object satisfying `materialized_page_contract`.
- The adapter path still fails closed when live execution is disabled.
- URL precheck and final URL recheck are both represented in the response.
- Raw HTML/artifacts are represented by refs only.
- Evidence handoff marks candidate state but does not promote unsupported content as source truth.

## Second Slice: Launch/Profile Compiler

This slice should not launch a browser yet. It should make the profile compiler precise enough that a future live adapter has no caller-controlled launch holes.

Syntax patterns to extract:

| Pattern | CloakBrowser Source | Infring Rule |
| --- | --- | --- |
| Deduplicate args by flag key | `js/src/args.ts::buildArgs` | Useful as pure compiler logic, but allowed args must come from policy, not caller. |
| Dedicated fields override defaults | `js/src/args.ts::buildArgs` | Only admitted policy/profile fields can override defaults. |
| Strip detectable context options | `js/src/playwright.ts::filterStealthCtxOptions` | Generalize to "reject or normalize conflicting adapter fields" in telemetry. |
| Cleanup on context creation failure | `js/src/playwright.ts::launchContext` | Live adapter must close browser if context creation fails. |
| Persistent context path | `js/src/playwright.ts::launchPersistentContext` | Defer; not part of stateless public materialization. |
| Humanize patch points | `js/src/playwright.ts::launch` / `launchContext` | Defer; not part of read-only materialization. |

Second slice exit criteria:

- Profile compiler returns an effective profile with source, denied field count, denied launch args, timeout, viewport, wait strategy, artifact budget, and cleanup obligations.
- Caller-provided denied fields remain rejected before profile compilation.
- Tests prove rejected fields never reach the adapter request.

## Third Slice: Adapter Execution Boundary

Only after the first two slices are done should we add a live local browser adapter.

The live adapter should be admitted by policy and exposed as a provider behind the existing API. The first implementation can be deliberately plain Playwright or an existing local browser capability; it does not need to reproduce CloakBrowser's patched binary or stealth claims.

Required output shape:

```json
{
  "source_url": "...",
  "final_url": "...",
  "status_code": null,
  "title": "...",
  "main_text_or_markdown": "...",
  "links_summary": [],
  "blocker_classification": {
    "blocker_class": "none|anti_bot_challenge|needs_js|rate_limited|access_denied|content_too_thin",
    "retryable": false,
    "evidence_impact": "usable|low_confidence_raw|rejected"
  },
  "extraction_confidence": "usable|low_confidence_raw|rejected",
  "artifact_ref": "..."
}
```

This JSON is an adapter contract example, not a user-visible answer format.

## Deferred Capability Slices

These remain out of the first Level 5 implementation:

| Capability | Why Deferred | Source Files |
| --- | --- | --- |
| Proxy support | Permission-sensitive and requires secret handling/redaction. | `js/src/proxy.ts`, `tests/test_proxy.py`, `js/tests/proxy.test.ts` |
| GeoIP/timezone consistency | Depends on proxy exit metadata and external geo DB lifecycle. | `js/src/geoip.ts`, `cloakbrowser/geoip.py`, geo tests |
| Persistent profiles | Creates identity retention and cleanup obligations. | `launchPersistentContext`, persistent-context tests |
| Humanized interaction | Interactive behavior is not needed for first read-only retrieval primitive. | `js/src/human/**`, `cloakbrowser/human/**`, humanize tests |
| CDP service pool | Useful for performance later, but expands authority and lifecycle surface. | `bin/cloakserve`, `tests/test_cloakserve.py` |
| Binary download/update | Heavy dependency lifecycle should not surprise-run during research. | `download.py`, `js/src/download.ts`, update tests |

## Test Plan By Slice

| Slice | Mock-Fast Tests | Live Tests |
| --- | --- | --- |
| One-shot materialization fake provider | URL precheck, final URL recheck, output contract, artifact quarantine, evidence handoff. | None. |
| Profile compiler | Arg dedupe, denied fields, context conflict normalization, timeout/budget bounds. | None. |
| Live adapter disabled | Enabled false, adapter missing, adapter ready stub, admission missing. | None. |
| Live adapter minimal | Fixture/static local page, JS-rendered local page, redirect-to-localhost block, timeout classification. | Localhost only, no anti-bot site. |
| Evidence packaging | HTML to main text, title/link extraction, blocker shell rejection, raw artifact hidden. | Local fixture page. |
| Golden impact | Gate diagnostics for browser-materialization lane separate from search/fetch. | Only after local fixture tests pass. |

## Open Design Choices

- Whether live browser execution should be implemented in Rust directly, through a small Node/Playwright helper, or through an external browser service adapter.
- Whether the first adapter should use the bundled browser/runtime available in Codex or require explicit installation/readiness.
- Whether the materialized page artifact should store raw HTML only, screenshot refs only, or both.
- How much extracted text is enough before promoting a materialized page to evidence.
- Whether redirect revalidation should reuse the fetch SSRF resolver exactly or add browser-specific DNS/final-URL diagnostics.

## Exit Criteria

Level 5 map is complete when:

- Every relevant CloakBrowser implementation/test file is listed in the burn-down.
- The first implementation slice is identified and bounded.
- Deferred stealth/proxy/session/human/service capabilities are separated from the first materialization primitive.
- The target Infring files and tests are named.
- The doc makes it impossible to claim "fully assimilated" without per-file status updates.

Level 5 implementation will be complete later, when:

- The one-shot materialization primitive has a fake provider and a minimal live provider behind policy admission.
- The materialized page output becomes an evidence candidate with raw payload refs.
- Local fixture tests prove JS-rendered content, final URL revalidation, timeout/blocker classification, cleanup, and evidence handoff.
- Research workflow gates can see browser-materialization diagnostics without making browser execution the default search path.
