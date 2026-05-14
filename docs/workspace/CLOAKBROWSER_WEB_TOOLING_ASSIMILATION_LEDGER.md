# CloakBrowser Web Tooling Assimilation Ledger

Created: 2026-05-13

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local source clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Goal

Track high-level CloakBrowser pattern assimilation for improving Infring web tooling, especially the current failure class where search/fetch providers return weak data, anti-bot signals, or off-topic fallback rows.

This ledger is intentionally about portable architecture and control-flow patterns. It is not a plan to copy CloakBrowser source, vendor its browser binary, or make stealth/proxy behavior an ambient default.

Level 2 behavioral contract map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL2_BEHAVIORAL_CONTRACT_MAP.md`

Level 3 mechanics/algorithm map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL3_MECHANICS_ALGORITHM_MAP.md`

Level 4 implementation-structure map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md`

Level 5 syntax implementation map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md`

## Guardrails

- Keep ordinary research retrieval provider-neutral and policy/CD-driven.
- Treat browser, stealth, proxy, persistent profile, and humanized interaction as gated capabilities, not default web-search behavior.
- Do not hardcode research domains, prompts, source lists, or output formats.
- Do not copy source-level Chromium patches or browser-distribution machinery into Infring.
- Preserve the distinction between retrieval quality, provider access failure, anti-bot detection, and synthesis quality.
- Keep raw browser traces, proxy details, page internals, and detection telemetry out of user-visible final answers unless synthesized at a high level.
- Prefer improving Tool CD, web-conduit, batch-query, and evidence-pack primitives over adding workflow-specific Rust behavior.

## Repository Shape

| Surface | Count / Notes |
| --- | --- |
| Total tracked files | 116 |
| Main languages | Python and TypeScript |
| Top-level implementation | `cloakbrowser/**`, `js/src/**` |
| Tests | `tests/**`, `js/tests/**` |
| Examples | `examples/**`, `js/examples/**` |
| Primary domain | Stealth Chromium wrapper for Playwright/Puppeteer plus binary/cache/update management |

## Target Overview

CloakBrowser is a drop-in browser automation wrapper around a patched Chromium binary. Its strongest architectural patterns are not search ranking or document extraction; they are browser-provider admission, launch-context hygiene, proxy/geolocation consistency, behavioral interaction realism, binary lifecycle management, and testable anti-detection contracts.

For Infring, this maps to a future dynamic retrieval capability lane:

```text
batch_query/search candidates
-> access/blocker diagnostics
-> admitted browser retrieval capability, if policy allows
-> page materialization
-> evidence pack
-> synthesis
```

It should not become:

```text
all web research always uses stealth browser by default
```

## Architecture Mapping

| CloakBrowser Component | Target Purpose | Infring Destination | Fit | Notes |
| --- | --- | --- | --- | --- |
| `cloakbrowser/browser.py` / `js/src/playwright.ts` | Launch patched Chromium through familiar Playwright APIs. | `core/layer0/ops/src/web_conduit*` plus Tool CD capability manifest. | Adapt | Pattern is a narrow browser adapter with normal Playwright ergonomics. Infring should keep it behind `browser_fetch` or similar admitted capability. |
| `js/src/args.ts` / `cloakbrowser/config.py` | Compile stealth defaults, user args, timezone, locale, viewport, and platform args into a deduped launch contract. | Tool CD launch profile policy and Gateway adapter config. | Accept pattern | Centralized arg compilation prevents scattered browser flags and makes profile changes auditable. |
| `js/src/playwright.ts` context option filtering | Strip context-level locale/timezone because that path is more detectable; prefer top-level binary/profile settings. | Browser retrieval adapter request normalization. | Accept pattern | General lesson: normalize/deny conflicting options at the adapter boundary and explain internally. |
| `js/src/proxy.ts` / proxy tests | Parse proxy URLs, separate credentials, handle SOCKS as browser CLI args, normalize special characters. | Secret/permission-gated proxy capability. | Accept pattern with guard | Useful for robust config parsing, but proxy usage remains capability-gated and never ambient. |
| `js/src/geoip.ts` / `cloakbrowser/geoip.py` | Resolve proxy exit IP, timezone, locale, and WebRTC IP consistency. | Browser retrieval profile metadata and anti-bot diagnostics. | Defer | Useful only once proxy/browser escalation is admitted. Do not add as ordinary search. |
| `js/src/human/**` / `cloakbrowser/human/**` | Humanized mouse, keyboard, scroll, isolated-world DOM reads, and trusted key dispatch. | Future dynamic page interaction lane. | Defer | Useful for interactive pages and bot-wall diagnostics. Keep off by default and budgeted. |
| `launch_persistent_context` / persistent context tests | Reuse browser profile state across sessions. | Browser session manager / capability handle. | Defer | Can help pages that punish incognito/fresh sessions, but introduces retention, identity, and cleanup obligations. |
| `bin/cloakserve` / `tests/test_cloakserve.py` | Expose browser access as a CDP service with per-connection identity/profile parameters. | Optional browser materialization service behind Gateway/web-conduit. | Defer | Useful pattern for pooling/session handles and connection accounting. Do not expose ambient remote debugging authority to workflows. |
| AWS Lambda example and `tests/test_lambda_security.py` | One-shot browser materialization endpoint with URL validation, post-redirect validation, retry classification, and hardening flags. | Browser materialization security contract. | Accept pattern | Browser fetch needs SSRF-safe URL validation, scheme allowlists, redirect revalidation, caller-arg filtering, and bounded retry strategy. |
| `download.py` / `js/src/download.ts` | Binary versioning, cache, checksum, primary/fallback download, background update. | Provider runtime dependency management. | Accept pattern only | Infring should not silently download a browser binary during normal research. The lifecycle pattern is useful for optional provider readiness. |
| `tests/test_extract.py` | Archive extraction hardening: path traversal checks, symlink handling, permission normalization, flattening rules. | Runtime dependency extraction/install guards. | Accept pattern only | Applies if optional browser-provider readiness ever installs or extracts local artifacts. Not part of normal research retrieval. |
| Python + JS wrappers and tests | Keep Playwright/Puppeteer/Python APIs semantically aligned. | Cross-adapter provider contract tests. | Accept pattern | If Infring exposes multiple retrieval adapters, they need one capability contract with adapter-specific implementations, not drifting semantics. |
| `tests/test_stealth_unit.py` / JS tests | Mock-fast tests for isolated-world lifecycle, key dispatch, proxy parsing, args, launch contracts. | Web tooling provider contract tests. | Accept | Great test shape: verify browser-provider contracts without requiring live browser access for every test. |
| README test-result matrix | Public evidence of blocked/unblocked detection sites. | Assimilation context only. | Reject as proof | Do not import marketing claims as Infring evidence. Treat as leads for capability design. |

## Assimilation Targets

| ID | Target | Why It Matters | Infring Destination | Status |
| --- | --- | --- | --- | --- |
| `CLOAK-PATTERN-001` | Browser retrieval as an admitted escalation lane | Current providers hit anti-bot/circuit-open states; we need a clean way to try browser materialization without making it default. | Tool CD + web-conduit provider registry | contract integrated |
| `CLOAK-PATTERN-002` | Provider health and anti-bot state split | `anti_bot_challenge`, provider degradation, and low-signal SERP junk should not collapse into one "bad data" bucket. | Web tooling diagnostics and retrieval health gates | decision lattice integrated |
| `CLOAK-PATTERN-003` | Launch/profile contract compiler | Browser retrieval needs a deterministic profile object instead of ad hoc flags. | Tool CD browser profile schema | contract integrated |
| `CLOAK-PATTERN-004` | Proxy parsing and secret separation | If proxy capability is admitted later, URL strings, credentials, SOCKS, bypass lists, and logs must be handled safely. | Gateway secret broker + Tool CD proxy capability | queued |
| `CLOAK-PATTERN-005` | Geo/proxy consistency metadata | Some blocked pages depend on locale/timezone/IP/WebRTC consistency. | Browser retrieval capability metadata | deferred |
| `CLOAK-PATTERN-006` | Humanized interaction as budgeted action primitive | Some pages need clicks/scrolls/typing; those should be explicit actions with time budgets and telemetry. | Dynamic page interaction adapter | deferred |
| `CLOAK-PATTERN-007` | Isolated-world DOM reads | Page state reads can avoid main-world interference and be tested independently. | Browser fetch/extract adapter internals | deferred |
| `CLOAK-PATTERN-008` | Persistent context/session lifecycle | Session reuse can improve access but creates retention and cleanup obligations. | Browser session capability handles | deferred |
| `CLOAK-PATTERN-009` | Browser dependency readiness lifecycle | Optional heavy providers need installed/version/available/update/cleanup status. | Runtime provider readiness diagnostics | default-off status integrated |
| `CLOAK-PATTERN-010` | Mock-fast provider contract tests | Browser capability tests should not require live blocked sites every run. | Web tooling tests | contract tests integrated |
| `CLOAK-PATTERN-011` | Browser materialization security envelope | Dynamic browser fetch expands SSRF, redirect, local-file, and caller-argument risks. | Browser materialization Tool CD + Gateway/web-conduit adapter | contract integrated |
| `CLOAK-PATTERN-012` | Browser service/session pool as optional provider | CDP service mode can amortize launch cost and isolate identities by seed/session. | Future browser provider service | deferred |
| `CLOAK-PATTERN-013` | Cross-adapter capability parity | Python, Playwright, Puppeteer, and service adapters should satisfy one provider contract. | Provider contract tests | queued |

## Accepted Patterns

| Pattern | Assimilation Decision | System Mapping |
| --- | --- | --- |
| Narrow adapter over familiar browser APIs | Accept | Add or extend a browser-materialization provider behind web-conduit, not inside the research workflow CD. |
| Compile launch/profile settings centrally | Accept | Tool CD should declare browser profile knobs, defaults, denied/conflicting fields, and artifact shape. |
| Separate configured capability from runtime readiness | Accept | Provider status should expose installed/version/credential/blocked/circuit state without user-visible trace leakage. |
| Detect and classify anti-bot states distinctly | Accept | Web tooling gates should keep `anti_bot_or_throttle` separate from `provider_empty`, `low_signal`, and `content_materialization_missing`. |
| Validate browser materialization targets before and after navigation | Accept | Browser-backed fetch must reject unsafe schemes, private/internal networks, and redirect targets before content enters the evidence path. |
| Filter caller-supplied launch/runtime arguments | Accept | Tool callers should not be able to inject remote debugging, certificate, file access, or unsafe browser flags; recovery strategy args are internal only. |
| Classify retryable navigation failures | Accept | Timeout/cert/network classes can drive bounded provider retries without leaking raw trace text to final answers. |
| Normalize proxy credentials at the boundary | Accept with permission gate | If proxy support is added, credentials live in Gateway/secret state and projected diagnostics must be redacted. |
| Keep humanized interaction parameters centralized | Accept as future gated primitive | Behavioral knobs belong in a capability profile, not scattered script code. |
| Mock unit tests for browser semantics | Accept | Tests can lock launch profile filtering, proxy parsing, and anti-bot diagnostics without hitting live sites. |
| Keep adapter APIs equivalent across runtimes | Accept | Capability semantics should be tested once and projected onto language/runtime adapters, preventing drift between implementations. |

## Rejected Or Deferred Patterns

| Pattern | Decision | Rationale |
| --- | --- | --- |
| Make stealth browser the default web research provider | Reject | Too heavy, policy-sensitive, and unnecessary for ordinary pages. It would hide provider quality failures instead of measuring them. |
| Copy/source-level Chromium patches | Reject | This is not a portable Infring primitive and would create large maintenance/security surface. |
| Download browser binaries during ordinary research runs | Reject for default path | Optional provider readiness can install/check dependencies, but normal research should not surprise-download heavy artifacts. |
| Expose raw CDP/debugging ports as workflow authority | Reject | Service-mode browser access must remain behind Gateway/tool admission, with bounded sessions and redacted diagnostics. |
| Use proxy rotation as default recovery | Reject | Proxy behavior is permission-sensitive and must be explicit/admitted. |
| Treat marketing/test claims as evidence | Reject | Use repo claims as design leads only. Live Infring tests must prove capability. |
| Persist browser sessions by default | Defer | Useful but introduces identity, privacy, cleanup, cache, and retention complexity. |

## Parsed High-Level Files

| File | Status | Pattern Signal |
| --- | --- | --- |
| `README.md` | parsed | Product positioning, capability taxonomy, stealth/browser/anti-bot claims, integration vocabulary. |
| `cloakbrowser/browser.py` | parsed | Python launch/context wrapper, backend selection, proxy resolution, geoip, cleanup-on-close, humanize hook. |
| `js/src/playwright.ts` | parsed | TypeScript launch/context wrapper, context option filtering, geoip/WebRTC consistency, humanize patch points. |
| `js/src/args.ts` | parsed | Deduped argument compiler with explicit override order. |
| `js/src/config.ts` | parsed | Platform detection, binary cache path, version map, ignored default args, default stealth args. |
| `js/src/proxy.ts` | parsed | Robust proxy URL parsing, SOCKS handling, credential normalization, pass-through fallback. |
| `js/src/geoip.ts` | parsed | Proxy exit IP resolution, timezone/locale inference, bounded timeout, optional dependency behavior. |
| `js/src/human/config.ts` | parsed | Centralized human interaction presets and action timing knobs. |
| `js/src/human/mouse.ts` | parsed | Bezier mouse movement, wobble, overshoot, burst pauses, click targeting, idle drift. |
| `js/src/human/index.ts` | parsed | Method patching, isolated-world DOM reads, cursor state, trusted key dispatch support. |
| `tests/test_launch.py` | parsed | Launch and basic anti-detection invariant tests. |
| `tests/test_proxy.py` | parsed | Proxy parsing and GeoIP behavior tests. |
| `tests/test_stealth_unit.py` | parsed | Isolated-world lifecycle and stealth interaction unit tests without live browser dependency. |
| `bin/cloakserve` | parsed | CDP multiplexer, per-seed browser process pool, safe data-dir deletion, port allocation, connection refcounting. |
| `tests/test_cloakserve.py` | parsed | Query/CLI parsing, URL rewriting, connection tracking, remote-debugging flag stripping. |
| `examples/integrations/aws_lambda/lambda_handler.py` | level 5 pass 001 integrated | Browser materialization endpoint, URL validation, smart DOM settle wait, retry strategy classification, launch hardening, final URL revalidation, cleanup, and telemetry-only retry history. |
| `tests/test_lambda_security.py` | parsed | Scheme allowlist, SSRF/private IP rejection, redirect revalidation, caller argument filtering, hardening flags. |
| `tests/test_extract.py` | parsed | Dependency extraction hardening: archive traversal checks, symlink handling, flattening, executable permissions. |

## Source Inventory Snapshot

This is a high-level pass, not a full repo burn-down.

| Surface | Files | State | Notes |
| --- | ---: | --- | --- |
| Python package `cloakbrowser/**` | 15 | partially parsed | Core launch, config, proxy/geo, human behavior surfaces identified. |
| TypeScript package `js/src/**` | 33 | partially parsed | Playwright/Puppeteer wrappers, proxy, geoip, human behavior, download lifecycle. |
| Tests `tests/**` and `js/tests/**` | 21+ | partially parsed | Launch/proxy/stealth/lambda/cloakserve/extract tests parsed; remaining tests should be used for implementation-level edge-case confirmation. |
| Examples | 17 | not parsed | Useful later for capability demos, but lower value than implementation/tests. |
| Images/binary docs | 6 images + license/docs | skipped for now | Marketing/proof artifacts are not implementation patterns. |
| CI/package metadata | remainder | not parsed | Useful only for binary readiness and distribution lifecycle if we target that next. |

## Integration Strategy

### Short-Term

1. Extend web tooling diagnostics to report provider access blockers separately from weak result quality.
2. Add a Tool CD concept for `browser_materialize_page` as an optional capability, even if the first implementation is just a stub/provider contract.
3. Add readiness/status projection for browser-capable retrieval providers: installed, unavailable, missing dependency, permission needed, blocked, or circuit open.
4. Tighten fallback promotion so dictionary/reference/off-topic rows do not become evidence simply because search providers failed.

### Medium-Term

1. Add a browser fetch/materialization adapter behind web-conduit.
2. Keep launch profile options declarative: headless, viewport, locale, timezone, persistent profile, user data dir, proxy capability ref, and interaction budget.
3. Add blocker-aware retry policy: if `anti_bot_or_throttle` is detected and capability policy allows, try browser materialization; otherwise fail cleanly with structured low-evidence state.
4. Add browser-provider tests using fixtures/mocks for launch profile compilation, denied option filtering, proxy redaction, URL safety, redirect safety, and cleanup.
5. Add a browser materialization output contract: final URL, status/blocker classification, title, main text/markdown, links, screenshot/detail ref if enabled, and extraction confidence.

### Deferred

1. Humanized interaction beyond scrolling/clicking.
2. Persistent browser sessions.
3. Proxy/GeoIP consistency.
4. Any source-level browser patching or bundled binary lifecycle.

## Compatibility And Risk

| Risk | Description | Mitigation |
| --- | --- | --- |
| Authority bleed | Browser/proxy choices could sneak into workflow logic. | Keep in Tool CD + Gateway/web-conduit adapter; workflow sees capability outcome only. |
| Privacy/identity retention | Persistent sessions and proxy profiles carry identity state. | Disabled by default; require explicit capability handle, TTL, cleanup, and redacted telemetry. |
| Heavy runtime dependency | Browser binaries are large and slow. | Optional provider readiness, not default runtime path. |
| Security surface | Browser execution and proxy handling expand attack surface. | URL validation, SSRF controls, scheme allowlist, timeout/budget caps, no user-supplied scripts by default. |
| Anti-bot policy ambiguity | Circumventing access controls can be inappropriate. | Use only for permitted public retrieval contexts; record blockers and capability admission separately. |
| Measurement distortion | A stealth lane could hide core search-provider weakness. | Keep search gates and browser-materialization gates separate. |

## Priority Backlog

| ID | Status | Priority | Item | Destination | Dependencies | Notes |
| --- | --- | --- | --- | --- | --- | --- |
| `CLOAK-TASK-001` | integrated | high | Add explicit `browser_materialization_available` / `browser_materialization_attempted` diagnostic lane. | Web tooling diagnostics | Existing web gate split | Added default-off status and gate diagnostics for browser-materialization recovery visibility. |
| `CLOAK-TASK-002` | integrated | high | Add Tool CD stub for browser page materialization capability. | Tool CD registry | Tool CD shape | Declared inputs, outputs, budgets, side effects, security, and permission class. |
| `CLOAK-TASK-003` | queued | high | Strengthen provider fallback filtering to reject lexical-definition/off-topic rows. | Batch-query candidate filtering | Current low-signal diagnostics | Directly targets observed Merriam-Webster/MDN bad evidence. |
| `CLOAK-TASK-004` | integrated | medium | Add provider readiness status for optional browser capability. | web-conduit status | Capability manifest | Status now projects default-off browser materialization capability without installing or executing anything. |
| `CLOAK-TASK-005` | integrated | medium | Add mock tests for anti-bot -> optional browser escalation decision. | Web tooling tests | `CLOAK-TASK-001/002` | Added fixture-backed tests that classify anti-bot output and surface browser-materialization recovery as telemetry-only capability guidance. |
| `CLOAK-TASK-006` | deferred | medium | Add proxy config parser/redaction contract if proxy capability is admitted. | Gateway/secret broker + Tool CD | Permission model | Do not implement before capability policy is explicit. |
| `CLOAK-TASK-007` | deferred | low | Evaluate humanized scroll/click as a dynamic page readiness primitive. | Browser interaction adapter | Browser materialization adapter | Only after basic browser fetch is stable. |
| `CLOAK-TASK-008` | integrated | high | Define browser materialization URL-safety contract. | Gateway/web-conduit + Tool CD | `CLOAK-TASK-002` | Scheme allowlist, DNS/IP safety, redirect revalidation, timeout budget, unsafe arg rejection. |
| `CLOAK-TASK-009` | queued | medium | Add cross-adapter provider contract tests. | Web tooling tests | Browser Tool CD stub | Same capability semantics across direct fetch, browser materialization, and future service adapters. |
| `CLOAK-TASK-010` | deferred | medium | Evaluate CDP service/browser pool pattern. | Future browser provider service | Basic browser materialization adapter | Useful for performance once the single-run provider is proven. |
| `CLOAK-TASK-011` | integrated | high | Add retrieval decision lattice diagnostics. | Web tooling diagnostics | Level 2 blocker taxonomy | Distinguishes synthesize, direct fetch, browser materialization, alternate provider, agent query refinement, and low-evidence terminal states. |
| `CLOAK-TASK-012` | integrated | high | Add query refinement signal payload. | Batch-query retry diagnostics | Candidate ranking and blocker taxonomy | Gives the agent preserve terms, candidate term hints, missing coverage buckets, blocker class, and strategy signals without hidden query generation. |
| `CLOAK-TASK-013` | integrated | high | Add evidence promotion metadata. | Evidence pack | Candidate scoring and safety hints | Evidence rows now show promotion decision, safety state, scoring components, caveats, and raw-payload chat boundary. |
| `CLOAK-TASK-014` | integrated | medium | Add provider normalization report. | Retrieval broker | Provider attempts | Broker now separates provider normalization status from evidence quality and synthesis readiness. |
| `CLOAK-TASK-015` | integrated | high | Add retry stop-condition diagnostics. | Retrieval broker | Retrieval decision lattice | Broker now reports stop/continue state, observed budgets, quality state, and capability requirements without executing hidden retries. |
| `CLOAK-TASK-016` | integrated | high | Add artifact quarantine diagnostics. | Retrieval broker and evidence pack | Evidence promotion metadata | Broker now reports raw artifact refs as quarantined and confirms raw payloads are not chat-visible. |
| `CLOAK-TASK-017` | integrated | high | Add candidate URL safety diagnostics. | Web tooling diagnostics | Browser materialization contract | Candidate refs now expose materialization URL safety status and block internal/credentialed/non-HTTP(S) locators. |
| `CLOAK-TASK-018` | integrated | medium | Add browser profile compilation diagnostics. | Browser materialization diagnostics | Tool CD profile contract | Diagnostics now project default-off profile source, denied caller fields, and separately admitted capabilities. |
| `CLOAK-TASK-019` | integrated | medium | Add page readiness and extraction handoff diagnostics. | Retrieval broker | Provider results and evidence pack | Broker now separates evidence packaged, blocker shell, thin extraction, fetch/materialization-needed, and not-observed states. |
| `CLOAK-TASK-020` | integrated | medium | Add optional browser readiness lifecycle diagnostics. | Browser materialization diagnostics | Dependency readiness lifecycle | Diagnostics now report default-off readiness state and prevent missing browser dependencies from being mistaken for search quality. |
| `CLOAK-TASK-021` | integrated | high | Create Level 4 implementation-structure map. | Assimilation docs and ownership ledger | Level 3 closure | Maps each CloakBrowser-derived mechanic to a concrete CD/module/test owner before live adapter work. |
| `CLOAK-TASK-022` | integrated | high | Enforce URL-credential rejection at fetch preflight. | web-conduit SSRF guard + Tool CD/policy vocabulary | `CLOAK-L4-003` | Fetch-boundary safety now matches candidate/materialization diagnostics before any provider execution. |
| `CLOAK-TASK-023` | integrated | medium | Project browser profile compilation at runtime. | Provider runtime metadata | `CLOAK-L4-004` | Runtime status now compiles the default profile, denied caller fields, denied launch args, and chat-hidden trace boundary without launching a browser. |
| `CLOAK-TASK-024` | integrated | medium | Surface browser profile/readiness state in effective inventory. | Web tooling inventory | `CLOAK-L4-004/006` | Inventory consumers can now see the optional browser lane's profile compilation status and readiness lifecycle without knowing deep runtime metadata paths. |
| `CLOAK-TASK-025` | integrated | high | Add default-off browser materialization API boundary. | web-conduit API/CLI + tests | `CLOAK-L4-003/004/005/006` | `api_browser_materialize_page` validates URL/admission fields, rejects caller browser controls, reuses SSRF safety, and fails closed until an admitted adapter exists. |
| `CLOAK-TASK-026` | integrated | medium | Project materialized-page output and evidence handoff contracts. | web-conduit API + tests | `CLOAK-TASK-025` | Browser materialization responses now expose the page-output schema, evidence promotion requirements, and artifact quarantine state without creating raw payloads. |
| `CLOAK-TASK-027` | integrated | high | Create Level 5 syntax implementation map. | Assimilation docs | Level 4 closure | Adds the file-by-file burn-down and first live-adapter slice plan so syntax assimilation can proceed one source file at a time. |
| `CLOAK-TASK-028` | integrated | high | Complete Level 5 pass 001 for Lambda one-shot materialization syntax. | Browser materialization contract and tests | `CLOAK-TASK-027` | Added boundary diagnostics for pre-navigation URL safety, final URL safety, navigation/readiness strategy, cleanup status, and retry recommendations without enabling live browser execution. Next step is security-test pass 002. |

## Open Questions

- Should browser materialization live as a separate Tool CD or as a capability mode inside `web_fetch`?
- What permission boundary is required before any proxy/session/humanized interaction capability is admitted?
- Do we want a local installed-browser readiness check first, before any attempt to run or install a stealth browser?
- Should anti-bot access failures count as hard retrieval failures or soft quality failures in the golden dataset?
- What is the minimal page materialization output shape needed by synthesis: markdown, main text, title/metadata, links, status, blocker classification, and claim hints?

## Current Assessment

CloakBrowser is most useful to us as an architecture source for a future gated browser-materialization provider, not as a search engine replacement. It does not solve candidate discovery by itself; it helps when we already have a candidate URL or when a normal fetch/search provider is blocked by browser-detection defenses.

The immediate ROI remains:

1. Better provider/blocker diagnostics.
2. Better rejection of off-topic fallback rows.
3. A clean browser-materialization capability contract so anti-bot recovery can be added without contaminating the research workflow CD.

## Assimilation Wave 1: Capability Contract And Diagnostics

Status: integrated and narrowly tested.

Implemented:

- Added a default-off `browser_materialization` capability policy under the web conduit configuration and default policy.
- Declared `browser_materialize_page` in the web retrieval Tool CD with request, output, safety, admission, and non-goal contracts.
- Exposed browser materialization as an optional web tooling catalog/status row, with `capability_not_enabled` treated as a normal default-off state rather than a blocked core tool.
- Added telemetry-only browser-materialization recovery diagnostics when web output shows anti-bot, access-blocked, degraded-provider, or materialization-missing signals.
- Added a web retrieval diagnostic gate for visibility into whether a blocker recovery lane is present.
- Added fixture-backed tests for anti-bot recovery guidance and default-off status/catalog projection.

Validation:

- `jq empty core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `cargo test -p infring-ops-core status_bootstraps_default_policy_and_receipts_surface -- --nocapture`
- `cargo test -p infring-ops-core web_quality_diagnostics_tests::anti_bot_failures_emit_structured_quality_retry -- --nocapture`
- `git diff --check` on the touched CloakBrowser assimilation files

Important boundary:

This wave does not add a live browser executor. It creates the admitted capability shape, status surface, recovery diagnostic, and tests so a future executor can be added without making browser retrieval the default path or leaking raw browser traces into chat.

## Assimilation Wave 2: Level 2 Behavioral Contracts

Status: integrated and narrowly tested.

Implemented:

- Extended the browser materialization policy/CD contract with normalized request fields, denied caller fields, stateless profile defaults, denied launch flags, redirect and URL-credential safety, output artifact refs, and evidence-handoff requirements.
- Projected those same behavioral contracts through web-conduit public contracts and runtime web tooling metadata so the dashboard/status/catalog path can see the capability shape without receiving execution authority.
- Added blocker taxonomy diagnostics that separate anti-bot, JavaScript-required, rate-limit, access-denied, provider-degraded, content-materialization-missing, off-intent, and low-signal states.
- Kept retry authority with the agent and avoided making thin-but-usable evidence automatically retry just because the snippet is short.
- Added mock-fast tests for blocker taxonomy splitting and expanded the default-off browser-materialization catalog/status test.

Validation:

- `jq empty core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `cargo test -p infring-ops-core status_bootstraps_default_policy_and_receipts_surface -- --nocapture`
- `cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`
- `git diff --check` on the CloakBrowser Level 2 touched files

Important boundary:

This wave still stops before mechanics/syntax-level browser execution. Browser materialization remains default-off and optional; proxy, persistent sessions, humanized interaction, and service pooling remain separately gated future capabilities.

## Assimilation Wave 3: Level 3 Decision Mechanics

Status: integrated and narrowly tested.

Implemented:

- Added `retrieval_decision_lattice_v1` to web quality diagnostics.
- Added candidate URL state classification so browser materialization only targets concrete candidate URLs, while provider-level anti-bot/JavaScript blockers without a candidate route to alternate-provider or browser-capable retrieval admission.
- Added `query_refinement_signals_v1` under retry diagnostics, preserving agent authority over actual query text.
- Separated retrieval failure classes from answer-shape guidance: weak single-source and comparison coverage gaps now suggest agent query refinement, not browser escalation.
- Bumped web quality diagnostics to `web_tool_quality_v4` so stale cached diagnostics do not hide the new lattice fields.
- Added mock-fast tests for anti-bot blocker recovery, JavaScript/rate/access blocker splitting, direct fetch, weak single-source refinement, degraded provider fallback, and ready-for-synthesis paths.

Validation:

- `git diff --check -- core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`

Important boundary:

This wave still does not add live browser execution, proxy/session behavior, hidden query generation, or domain-specific research prompts. It makes the existing tooling less black-box by exposing what the tool thinks the next retrieval class is and why.

## Assimilation Wave 4: Evidence Promotion And Provider Normalization

Status: integrated and narrowly tested.

Implemented:

- Added `evidence_promotion_v1` metadata to evidence pack rows.
- Added source-safety hints for HTTP/HTTPS, credentialed URLs, internal-host locators, and raw-payload chat visibility.
- Added promotion decisions: `promoted`, `promoted_with_caveats`, and `retained_low_confidence`.
- Added `provider_normalization_v1` to the retrieval broker so provider attempts expose normalized status/phase/failure-class counts.
- Added tests proving clean evidence is promoted, unsafe/internal candidate locators are caveated, and provider degradation survives normalization as a provider failure class.

Validation:

- `git diff --check -- core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`

Important boundary:

This wave improves evidence and provider observability. It does not add a browser executor, proxy behavior, persistent session behavior, or source-specific research prompting.

## Assimilation Wave 5: Retry Stop Conditions And Artifact Quarantine

Status: integrated and narrowly tested.

Implemented:

- Added `retry_stop_conditions_v1` to the retrieval broker diagnostics.
- Added explicit stop states for ready synthesis, structured low evidence, exhausted query-refinement budget, and observe-only cases.
- Added explicit continue states for alternate provider, direct fetch, browser materialization, and agent query refinement when those moves are still useful.
- Added `artifact_quarantine_v1` to the retrieval broker diagnostics.
- Counted raw artifact-like refs across provider results, evidence packs, and tool-result-quality metadata while keeping `raw_payload_chat_visible` false.
- Projected evidence-promotion decisions through the quarantine report so promoted evidence can be audited without exposing raw bodies.
- Added mock-fast assertions for alternate-provider continuation, synthesis stop state, artifact quarantine, and promotion projection.

Validation:

- `git diff --check -- core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs docs/workspace/CLOAKBROWSER_LEVEL3_MECHANICS_ALGORITHM_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`

Important boundary:

This wave still does not add live browser execution, hidden retry generation, proxy behavior, or a research-domain prompt. It exposes whether the broker should stop or continue and whether raw artifacts stayed quarantined.

## Assimilation Wave 6: URL Safety, Profile Contract, And Extraction Readiness

Status: integrated and narrowly tested.

Implemented:

- Added `url_safety_assessment_v1` for candidate locators.
- Blocked browser-materialization candidate state when the only candidate URL is internal, credentialed, or non-HTTP(S).
- Added URL-safety status to retrieval decision candidate refs and evidence-promotion safety metadata.
- Added `browser_profile_compilation_v1` under browser-materialization diagnostics, including denied caller fields and separately admitted proxy/session/interaction/pool capabilities.
- Added `page_readiness_extraction_v1` to the retrieval broker to separate extraction readiness from provider retrieval health.
- Added `browser_capability_readiness_lifecycle_v1` so optional browser dependency state is observable without surprise installs or launches.
- Added mock-fast assertions for safe public URL candidates, internal/credentialed URL blocking, default-off profile compilation, and page readiness/extraction projection.

Validation:

- `git diff --check -- core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs docs/workspace/CLOAKBROWSER_LEVEL3_MECHANICS_ALGORITHM_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -p infring-ops-core web_quality_diagnostics_tests -- --nocapture`

Important boundary:

This wave remains diagnostic/contract-level. It does not add live browser navigation, proxy handling, persistent sessions, humanized interaction, or raw DOM extraction.

## Assimilation Wave 7: Level 4 Implementation Structure Map

Status: integrated as ownership map.

Implemented:

- Created `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md`.
- Mapped each Level 3 mechanic to a concrete Level 4 owner surface: Tool CD, web conduit, batch query, artifact store, workflow CD, Gateway/Kernel policy, Assurance/evals, or Shell projection.
- Named implementation targets for Tool CD linkage, web-conduit policy, URL safety, profile compilation, browser materialization, provider readiness, page extraction, evidence packaging, artifact quarantine, workflow CD integration, and eval ownership.
- Defined a phase order that starts with CD metadata and URL-safety parity before any live browser adapter.
- Preserved the boundary that browser materialization remains default-off and capability-admitted, with proxy/session/humanized/service-pool behavior deferred.

Validation:

- `git diff --check -- docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md docs/workspace/CLOAKBROWSER_LEVEL3_MECHANICS_ALGORITHM_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Important boundary:

This wave does not add runtime behavior. It exists to prevent the next runtime wave from scattering browser-specific authority across workflow Rust, shell state, or prompt-specific code.

## Assimilation Wave 8: Level 4 CD And URL-Safety Parity

Status: integrated and narrowly tested.

Implemented:

- Aligned browser-materialization URL safety status vocabulary in the Tool CD, checked-in web-conduit policy, and default policy.
- Added `blocked_url_credentials` and `blocked_internal_host_hint` to the admitted status vocabulary so the CD matches existing batch-query diagnostics.
- Added fetch-boundary credential detection to the web-conduit SSRF guard.
- Added `url_safety_status` to fetch SSRF guard output so fetch, redirect, and future browser-materialization paths can report a common safety state.
- Added focused tests proving credentialed URLs do not execute and guard safety status is projected.

Validation:

- `jq empty core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/020-fetch-policy-and-provider-contract-tests.rs core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib fetch_credentials_in_url_are_blocked_before_execution -- --nocapture`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib ssrf_guard_reports_redirect_target_safety_status -- --nocapture`

Important boundary:

This wave does not add live browser execution. It makes the existing static fetch boundary obey the same URL-safety contract that the browser-materialization lane will need.

## Assimilation Wave 9: Level 4 Browser Profile Policy Projection

Status: integrated and narrowly tested.

Implemented:

- Added `browser_profile_compilation_v1` to browser-materialization runtime metadata.
- Compiled Tool-CD/policy profile fields into one effective adapter envelope: profile source, default profile, state scope, denied caller fields, denied launch args, telemetry fields, and hidden raw-trace flags.
- Projected disabled, adapter-not-ready, and adapter-ready statuses without installing or launching a browser.
- Preserved separate admission boundaries for proxy, persistent sessions, caller-controlled launch args, raw CDP, and arbitrary user scripts.
- Added status-surface assertions so future browser adapter work has a visible contract to obey.
- Exposed profile compilation and readiness lifecycle state on the effective inventory row for the optional browser lane.

Validation:

- `git diff --check -- core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer0/ops/src/web_conduit_parts/041-tooling-inventory-and-policy.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/010-status-and-provider-catalog-tests.rs docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib status_bootstraps_default_policy_and_receipts_surface -- --nocapture`

Important boundary:

This wave still does not add browser execution, proxy handling, persistent sessions, humanized interaction, hidden retries, or final-answer formatting rules.

## Assimilation Wave 10: Default-Off Browser Materialization Boundary

Status: integrated and focused-tested.

Implemented:

- Added `api_browser_materialize_page` as the explicit web-conduit boundary for future browser-backed page materialization.
- Wired a `browser-materialize` CLI command to the same boundary so tests and operators can inspect the capability without launching a browser.
- Required URL and `admission_ref` fields before any execution path can proceed.
- Rejected caller-supplied browser controls such as launch args, raw CDP commands, user scripts, proxies, sessions, storage state, and local files.
- Reused the fetch SSRF/url-safety guard so credentialed, internal, invalid, or non-HTTP(S) targets fail before adapter execution.
- Preserved default-off behavior: disabled capability, missing adapter, and adapter stub states all fail closed with no browser launch and no chat-visible raw payload.
- Added mock-fast tests for default-off behavior, unsafe caller controls, credentialed URL blocking, and enabled-without-adapter readiness.

Validation:

- `git diff --check -- core/layer0/ops/src/web_conduit.rs core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/070-cli-run.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib browser_materialization -- --nocapture`

Important boundary:

This wave still does not add live browser execution, stealth behavior, proxy behavior, persistent sessions, humanized interaction, hidden retries, or browser installation. It creates a measurable primitive boundary that can later host an admitted adapter.

## Assimilation Wave 11: Materialized Page Output And Evidence Handoff Contract

Status: integrated and focused-tested.

Implemented:

- Added `materialized_page_contract` to browser materialization API responses so adapter work has a fixed output schema to satisfy.
- Added `evidence_handoff_contract` to responses, including candidate-enrichment target lane, promotion requirements, confidence vocabulary, and the rule that browser success is not source truth until packaged as evidence.
- Added `artifact_quarantine` projection to keep raw browser/page payload state explicit and non-chat-visible.
- Preserved fail-closed behavior for default-off, adapter-not-ready, and adapter-stub-only states.
- Added mock-fast coverage for output contract fields, evidence handoff, artifact quarantine, and ready-adapter stub behavior.

Validation:

- `git diff --check -- core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib browser_materialization -- --nocapture`

Important boundary:

This wave still does not create materialized page content. It makes the future adapter's output and evidence promotion obligations visible before live browser execution exists.

## Assimilation Wave 12: Level 5 Syntax Implementation Map

Status: integrated as planning and tracking doc.

Implemented:

- Created `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md`.
- Added a file-by-file Level 5 burn-down queue for CloakBrowser implementation files, tests, and examples.
- Defined the Level 5 completion rule: `parsed -> pattern extracted -> Infring target mapped -> integrated or rejected -> tested or deferred with reason`.
- Identified the first implementation slice as one-shot materialization security/extraction, grounded in the AWS Lambda handler and security tests.
- Separated deferred proxy, geo, persistent session, humanized interaction, CDP service pool, and binary lifecycle work from the first read-only browser materialization primitive.

Validation:

- `git diff --check -- docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Important boundary:

This wave is not live adapter implementation. It gives us the tracking surface needed to work through Level 5 one file at a time without accidentally claiming full assimilation from broad pattern reads.
