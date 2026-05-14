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
| `cloakbrowser/browser.py` | level 5 pass 011 integrated | Python launch/context wrapper, backend selection, proxy resolution, geoip, cleanup-on-close, async cancellation cleanup, persistent profile split, and humanize hook. |
| `js/src/playwright.ts` | level 5 pass 003 integrated | TypeScript launch/context wrapper, context option filtering, geoip/WebRTC consistency, humanize patch points, context cleanup, and persistent-session split. |
| `js/src/types.ts` | level 5 pass 004 integrated | Public launch/context/persistent-context API surface, direct browser/profile controls, storage/session fields, and binary readiness metadata. |
| `js/src/args.ts` | level 5 pass 005 integrated | Deduped argument compiler, fixed precedence, policy-owned profile arg assembly, and telemetry-only override visibility. |
| `js/src/config.ts` | level 5 pass 006 integrated | Platform detection, binary/cache/version readiness lifecycle, explicit install boundary, ignored default args, and deferred stealth defaults. |
| `js/src/download.ts` | level 5 pass 007 integrated | Atomic download, checksum verification, extraction hardening, cleanup on failure, and rate-limited update checks. |
| `cloakbrowser/download.py` | level 5 pass 013 integrated | Python operator install/update lifecycle, custom URL fallback suppression, platform-matched updates, binary-info redaction, and next-launch update markers. |
| `js/src/proxy.ts` | level 5 pass 008 integrated | Proxy URL parsing, credential separation, SOCKS adapter arg lane, credential encoding, bypass ownership, and redaction requirements. |
| `js/src/geoip.ts` | level 5 pass 009 integrated | Proxy exit IP resolution, timezone/locale inference, bounded timeout, optional dependency behavior, WebRTC IP consistency. |
| `cloakbrowser/geoip.py` | level 5 pass 014 integrated | Python GeoIP optional dependency, no first-use research DB download, exit-IP provider order, atomic DB replacement, and telemetry-only raw IP handling. |
| `js/src/puppeteer.ts` | level 5 pass 010 integrated | Cross-adapter launch parity, shared compiler/geo/proxy hooks, adapter-specific proxy auth patching, and humanize gate parity. |
| `cloakbrowser/config.py` | level 5 pass 012 integrated | Python provider defaults, per-platform browser version map, ignored default args, default viewport, cache/download paths, version markers, and local binary override boundaries. |
| `tests/test_cloakserve.py` | level 5 pass 016 integrated | Service query/CLI parsing, debugger URL rewrite, refcounting, seed validation, and data-dir cleanup containment invariants. |
| `js/src/human/config.ts` | parsed | Centralized human interaction presets and action timing knobs. |
| `js/src/human/mouse.ts` | parsed | Bezier mouse movement, wobble, overshoot, burst pauses, click targeting, idle drift. |
| `js/src/human/index.ts` | parsed | Method patching, isolated-world DOM reads, cursor state, trusted key dispatch support. |
| `tests/test_launch.py` | level 5 pass 017 integrated | Launch/close/page navigation contract, sync/async parity, binary-info telemetry, and probe-result quarantine. |
| `tests/test_launch_context.py` | level 5 pass 018 integrated | Context option lane separation, policy-owned viewport/profile fields, locale/timezone CDP-emulation denial, storage-state capability boundary, and cleanup expectations. |
| `tests/test_build_args.py` | level 5 pass 019 integrated | Argument compiler dedupe, dedicated field precedence, alias consumption, non-value flag admission, and WebRTC/fingerprint arg quarantine. |
| `tests/test_backend.py` | level 5 pass 020 reviewed, already covered | Backend resolution default/param/env/invalid cases; no new code because policy-owned adapter selection and direct backend rejection were already integrated. |
| `tests/test_config.py` | level 5 pass 021 integrated | Platform-specific binary/archive/cache defaults, operator-only fallback URL/cache overrides, unsupported-platform fail-closed behavior, and stealth seed/GPU flag quarantine. |
| `tests/test_proxy.py` | level 5 pass 022 integrated, deferred capability | Proxy parsing, credential separation, SOCKS/SOCKS5H adapter lane, encoding idempotence/redaction, malformed/nonstandard URL rejection, IPv6 preservation, and proxy/GeoIP capability boundaries. |
| `tests/test_geoip.py` | level 5 pass 023 integrated, deferred capability | Proxy IP extraction telemetry, BCP47 country-locale map, explicit profile precedence, timeout/nonfatal GeoIP behavior, and private IP evidence quarantine. |
| `tests/test_stealth_unit.py` | parsed | Isolated-world lifecycle and stealth interaction unit tests without live browser dependency. |
| `bin/cloakserve` | level 5 pass 015 integrated | CDP multiplexer, per-seed browser process pool, safe data-dir deletion, port allocation, connection refcounting, debugger URL rewrite, and service admission boundary. |
| `tests/test_cloakserve.py` | parsed | Query/CLI parsing, URL rewriting, connection tracking, remote-debugging flag stripping. |
| `examples/integrations/aws_lambda/lambda_handler.py` | level 5 pass 001 integrated | Browser materialization endpoint, URL validation, smart DOM settle wait, retry strategy classification, launch hardening, final URL revalidation, cleanup, and telemetry-only retry history. |
| `tests/test_lambda_security.py` | level 5 pass 002 integrated | Scheme allowlist, SSRF/private IP rejection, redirect revalidation, caller argument filtering, hardening flags, uppercase scheme parsing, CGNAT, and IPv4-mapped IPv6 safety. |
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
| `CLOAK-TASK-029` | integrated | high | Complete Level 5 pass 002 for Lambda security tests. | Browser materialization contract and shared SSRF guard | `CLOAK-TASK-028` | Added mock-fast coverage for non-HTTP scheme rejection, private/internal targets, caller `extra_args` / `_strategy_args` denial, and case-insensitive HTTP authority parsing. |
| `CLOAK-TASK-030` | integrated | high | Complete Level 5 pass 003 for Playwright context boundary. | Browser materialization profile/context contract | `CLOAK-TASK-029` | Added context lifecycle contract metadata, denied direct Playwright/profile override fields, and kept persistent sessions, humanized interaction, proxy, and geo behavior behind future explicit capabilities. |
| `CLOAK-TASK-031` | integrated | high | Complete Level 5 pass 004 for TypeScript API surface audit. | Browser materialization request contract and Tool CD | `CLOAK-TASK-030` | Audited `LaunchOptions`, `LaunchContextOptions`, `LaunchPersistentContextOptions`, and `BinaryInfo`; denied direct public aliases such as raw args, stealth args, persistent profile dirs, storage state, and camelCase proxy/session/local-file fields. |
| `CLOAK-TASK-032` | integrated | high | Complete Level 5 pass 005 for TypeScript argument compiler. | Browser materialization profile compiler contract | `CLOAK-TASK-031` | Added a policy-owned argument compiler contract with flag-key dedupe, explicit precedence, raw caller-arg denial, and telemetry-only override traces. |
| `CLOAK-TASK-033` | integrated | medium | Complete Level 5 pass 006 for TypeScript config/defaults. | Browser materialization dependency readiness lifecycle | `CLOAK-TASK-032` | Added dependency lifecycle metadata for runtime-owned platform detection, policy-owned cache/install boundaries, no surprise downloads, internal binary/download paths, and unsupported-platform readiness state. |
| `CLOAK-TASK-034` | integrated | medium | Complete Level 5 pass 007 for TypeScript binary download lifecycle. | Browser materialization install/update readiness contract | `CLOAK-TASK-033` | Added install/update hardening metadata for temp downloads, partial cleanup, checksum verification, archive traversal rejection, atomic update markers, and no background updates during ordinary research. |
| `CLOAK-TASK-035` | integrated | medium | Complete Level 5 pass 008 for TypeScript proxy resolution. | Browser materialization proxy capability contract | `CLOAK-TASK-034` | Added future proxy capability metadata for credential separation, Gateway secret ownership, SOCKS adapter arg lanes, internal credential encoding, bypass policy ownership, and raw proxy redaction. |
| `CLOAK-TASK-036` | integrated | medium | Complete Level 5 pass 009 for TypeScript GeoIP consistency. | Browser materialization geo/proxy consistency contract | `CLOAK-TASK-035` | Added future geo consistency metadata for timeout-bounded exit-IP lookup, policy-owned GeoIP cache lifecycle, no surprise GeoIP downloads, explicit profile precedence, unresolved WebRTC auto-IP removal, and raw exit-IP redaction. |
| `CLOAK-TASK-037` | integrated | medium | Complete Level 5 pass 010 for TypeScript Puppeteer adapter parity. | Browser materialization cross-adapter contract | `CLOAK-TASK-036` | Added adapter parity metadata, denied direct backend/adapter selection fields, and required shared compiler/proxy/geo/human gates across future adapter families. |
| `CLOAK-TASK-038` | integrated | medium | Complete Level 5 pass 011 for Python wrapper lifecycle. | Browser materialization lifecycle contract | `CLOAK-TASK-037` | Added wrapper lifecycle metadata for sync/async parity, driver cleanup, async cancellation cleanup, persistent profile separation, backend policy ownership, timezone alias normalization, and raw profile/driver redaction. |
| `CLOAK-TASK-039` | integrated | medium | Complete Level 5 pass 012 for Python provider defaults. | Browser materialization default config contract | `CLOAK-TASK-038` | Added default config metadata for per-platform version selection, ignored default arg ownership, default viewport ownership, operator-only local binary/download hooks, fingerprint/stealth capability separation, and raw marker/download redaction. |
| `CLOAK-TASK-040` | integrated | medium | Complete Level 5 pass 013 for Python binary download lifecycle. | Browser materialization operator install/update contract | `CLOAK-TASK-039` | Added metadata for custom download fallback suppression, checksum manifest policy, platform-matched release updates, pre-network update timestamping, next-launch binary updates, and raw binary-info redaction. |
| `CLOAK-TASK-041` | integrated | medium | Complete Level 5 pass 014 for Python GeoIP lifecycle. | Browser materialization geo/proxy capability contract | `CLOAK-TASK-040` | Added metadata for optional GeoIP dependency state, no first-use GeoIP DB downloads during research, source admission, atomic DB replacement, policy-owned exit-IP echo provider order, raw proxy-host IP redaction, and nonfatal dependency failures. |
| `CLOAK-TASK-042` | integrated | medium | Complete Level 5 pass 015 for CDP service pool boundary. | Browser materialization service/pool capability contract | `CLOAK-TASK-041` | Added metadata for Gateway-admitted service mode, raw CDP denial, per-seed process isolation, seed validation, local port allocation, connection refcounting, data-dir-confined cleanup, profile override denial, and telemetry-only debugger URL handling. |
| `CLOAK-TASK-043` | integrated | medium | Complete Level 5 pass 016 for CDP service pool tests. | Browser materialization service/pool capability contract | `CLOAK-TASK-042` | Added metadata for workflow denial of generic fingerprint query params, explicit repeated-query policy, policy-owned service CLI/data-dir/headless/debug flags, remote-debugging passthrough stripping, and Gateway-owned WebSocket scheme resolution. |
| `CLOAK-TASK-044` | integrated | medium | Complete Level 5 pass 017 for basic launch tests. | Browser materialization launch execution contract | `CLOAK-TASK-043` | Added metadata for admitted-adapter launch requirements, close-after-capture, sync/async parity, page navigation not becoming evidence before packaging, binary-info telemetry, handle quarantine, and fingerprint probe telemetry-only boundaries. |
| `CLOAK-TASK-045` | integrated | medium | Complete Level 5 pass 018 for launch context tests. | Browser materialization context option contract | `CLOAK-TASK-044` | Added metadata for policy-owned viewport/user-agent/color-scheme lanes, locale/timezone CDP-emulation denial, proxy-gated GeoIP fills, generic context kwarg denial from workflows, and storage-state session capability requirements. |
| `CLOAK-TASK-046` | integrated | medium | Complete Level 5 pass 019 for build-args tests. | Browser materialization argument compiler contract | `CLOAK-TASK-045` | Added metadata for single effective flag per key, dedicated locale/timezone precedence, policy admission for non-value flags, timezone alias consumption, raw fingerprint/WebRTC arg denial, admitted proxy exit-IP dependency, and raw WebRTC IP redaction. |
| `CLOAK-TASK-047` | reviewed | low | Complete Level 5 pass 020 for backend resolution tests. | Browser materialization adapter parity contract | `CLOAK-TASK-046` | Reviewed backend default/explicit/env/invalid cases and confirmed they are already covered by policy-owned backend selection, direct backend request denial, invalid backend fail-closed semantics, and no live backend switching. |
| `CLOAK-TASK-048` | integrated | medium | Complete Level 5 pass 021 for config tests. | Browser materialization default config and dependency readiness contracts | `CLOAK-TASK-047` | Added metadata for policy-owned platform binary path templates, archive naming, fallback download URLs, operator-only cache/env overrides, unsupported-platform fail-closed behavior, operator-only random seed generation, GPU fingerprint flag quarantine, and cache-dir redaction. |
| `CLOAK-TASK-049` | integrated | medium | Complete Level 5 pass 022 for proxy tests. | Browser materialization proxy capability contract | `CLOAK-TASK-048` | Added metadata for schemeless proxy normalization after admission, credential removal from server URLs, username-only support, SOCKS5H support, idempotent credential encoding, redacted encoding notices, nonstandard SOCKS path/query rejection, IPv6 bracket preservation, and port-zero policy admission. |
| `CLOAK-TASK-050` | integrated | medium | Complete Level 5 pass 023 for GeoIP tests. | Browser materialization geo/proxy capability contract | `CLOAK-TASK-049` | Added metadata for literal proxy IP extraction as telemetry, invalid proxy GeoIP nonfatal behavior, BCP47 country-locale map requirements, fill-only-missing profile fields, exit-IP consistency even when profile fields are complete, timeout preservation, and private IP evidence quarantine. |

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

## Assimilation Wave 13: Level 5 API Surface Audit

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/types.ts` as the syntax-level source of CloakBrowser's public launch/context API surface.
- Mapped broad caller-facing launch fields to Infring policy-owned profile controls rather than user request authority.
- Added CloakBrowser public aliases such as raw `args`, `stealthArgs`, camelCase proxy/session/storage/local-file fields, and `userDataDir` to the browser materialization denial contract.
- Kept binary path/cache/download metadata in the runtime readiness lane, not in user-facing evidence or chat-visible output.
- Extended mock-fast materialization tests so these public API aliases fail before URL safety, adapter readiness, or browser launch.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not add live browser execution. It closes request-contract gaps before adapter work so future browser/profile/session/proxy capability admission has a clean primitive boundary.

## Assimilation Wave 14: Level 5 Argument Compiler Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/args.ts` as the syntax-level source of CloakBrowser's launch-argument compiler behavior.
- Preserved the useful primitive: centralize browser launch args, dedupe by Chromium flag key, and use an explicit precedence order.
- Replaced CloakBrowser's caller `options.args` override lane with an Infring policy/profile-owned compiler contract.
- Projected the argument compiler contract through default policy, runtime profile metadata, and the Tool CD.
- Added mock-fast assertions that the profile projection exposes the compiler pattern and keeps caller-supplied args disallowed.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not add a live profile compiler or launch a browser. It declares the contract that a future admitted adapter must satisfy and keeps raw caller launch arguments rejected.

## Assimilation Wave 15: Level 5 Dependency Lifecycle Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/config.ts` as the syntax-level source of CloakBrowser's platform/version/cache/download/default config behavior.
- Preserved the useful primitive: provider dependency readiness must expose platform support, binary/cache lifecycle, install boundaries, and cleanup ownership before execution.
- Rejected importing CloakBrowser's exact Chromium versions, download URLs, stealth defaults, random fingerprint seed behavior, or local binary override as ordinary research behavior.
- Added browser dependency lifecycle metadata to default policy, runtime readiness metadata, and the Tool CD.
- Added mock-fast assertions that browser materialization reports no surprise downloads and keeps raw binary path details out of chat-visible output.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not install, download, or launch a browser. It makes dependency readiness auditable so a future adapter cannot hide heavy runtime setup inside normal research.

## Assimilation Wave 16: Level 5 Installer Hardening Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/download.ts` as the syntax-level source of CloakBrowser's binary install, checksum, extraction, cleanup, and update mechanics.
- Preserved the useful primitive: any optional browser dependency lane must be atomic, checksum-verified, archive-hardened, cleanup-aware, and update-rate-limited.
- Kept installs, local binary overrides, primary/fallback download URLs, wrapper update notices, and background updates out of ordinary research execution.
- Extended browser dependency lifecycle metadata in default policy, runtime readiness metadata, and the Tool CD.
- Added mock-fast assertions for checksum requirement, archive traversal rejection, and no background updates during ordinary research.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not create an installer or live browser adapter. It records the install/update safety rules the future explicit readiness lane must satisfy.

## Assimilation Wave 17: Level 5 Proxy Capability Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/proxy.ts` as the syntax-level source of CloakBrowser's proxy URL parsing and SOCKS handling behavior.
- Preserved the useful primitive: proxy config is structured, credentials are separated from server URLs, special credentials are internally encoded, and SOCKS needs a dedicated adapter arg lane.
- Mapped credentials to Gateway/secret broker ownership and kept raw proxy URLs/credentials out of chat-visible outputs.
- Kept direct proxy request fields denied and left proxy use behind a separate future capability.
- Added mock-fast assertions that browser materialization exposes the proxy capability requirement and keeps raw credentials hidden.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not admit proxy use, proxy rotation, or proxy-based recovery. It only records the contract a future explicit proxy capability must satisfy.

## Assimilation Wave 18: Level 5 Geo Consistency Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/geoip.ts` as the syntax-level source of CloakBrowser's proxy exit-IP, GeoIP, locale/timezone, timeout, and WebRTC IP consistency behavior.
- Preserved the useful primitive: geo/location consistency is a capability-bound profile concern, not an ordinary research request field.
- Kept GeoIP DB downloads, raw DB paths, raw exit IPs, IP echo provider details, and WebRTC spoofing mechanics out of user-visible chat.
- Required no surprise GeoIP downloads during ordinary research and tied any future geo enrichment to policy-owned cache lifecycle.
- Added mock-fast assertions that direct geo request fields remain denied and raw exit IP is not chat-visible.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not admit geo spoofing, proxy behavior, WebRTC spoofing, or any live browser execution. It only records the contract a future explicit geo/proxy capability must satisfy.

## Assimilation Wave 19: Level 5 Adapter Parity Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `js/src/puppeteer.ts` as the syntax-level source of CloakBrowser's Puppeteer launch path and cross-adapter behavior.
- Preserved the useful primitive: multiple browser adapters must share one semantic contract for binary readiness, argument compilation, proxy handling, geo/WebRTC consistency, and humanized interaction gating.
- Rejected direct backend/adapter selection fields before adapter launch so callers cannot choose a lower-integrity path.
- Kept adapter-specific proxy authentication/page patch mechanics out of chat-visible output.
- Added mock-fast assertions that browser materialization exposes adapter parity metadata and rejects direct backend/adapter fields.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not add Puppeteer, Playwright, or any live browser backend. It only records the parity contract that future explicit adapters must satisfy.

## Assimilation Wave 20: Level 5 Wrapper Lifecycle Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `cloakbrowser/browser.py` as the syntax-level source of CloakBrowser's Python launch/context wrapper lifecycle.
- Preserved the useful primitive: browser adapter wrappers must maintain sync/async semantic parity and must close launched browser/driver state on close, context creation failure, and async cancellation.
- Kept persistent profile paths, backend selection, timezone aliases, raw driver handles, and raw profile paths out of ordinary request authority.
- Required persistent profile behavior to remain a separate admitted capability rather than part of stateless read-only materialization.
- Added mock-fast assertions for wrapper lifecycle metadata and direct `timezone_id` rejection.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not add live browser execution, persistent sessions, backend switching, or humanized interaction. It only records the lifecycle contract any future explicit adapter must satisfy.

## Assimilation Wave 21: Level 5 Default Config Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `cloakbrowser/config.py` as the syntax-level source of CloakBrowser's Python provider defaults, platform/version map, ignored default args, cache paths, download URLs, and local binary override behavior.
- Preserved the useful primitive: provider defaults and readiness knobs are policy-owned metadata, not caller-owned request parameters.
- Kept random fingerprint seeds, platform spoofing, ignored default args, local binary overrides, download URLs, cache roots, and version markers out of ordinary research authority.
- Added fail-closed rejection for direct default/config aliases such as `ignoreDefaultArgs`, `download_url`, and `fingerprintSeed`.
- Added mock-fast assertions for default config metadata and direct config field rejection.

Validation:

- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not add live browser execution, stealth defaults, local binary override, or any install/download behavior during ordinary research. It only records the policy contract for future explicit readiness/admission.

## Assimilation Wave 22: Level 5 Operator Install/Update Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `cloakbrowser/download.py` as the syntax-level source of CloakBrowser's Python binary readiness, download, checksum, extraction, binary-info, and update mechanics.
- Preserved the useful primitive: dependency install/update behavior belongs to explicit operator readiness lanes, while ordinary research gets only stable readiness/status diagnostics.
- Added contract metadata for custom download URLs requiring operator action, disabling public fallback, policy-owned checksum manifest lookup, platform-matched release assets, pre-network update timestamping, next-launch update markers, process-scoped wrapper update checks, and telemetry-only failures.
- Kept binary paths, cache dirs, download URLs, wrapper notices, update traces, local overrides, and install/download actions out of chat-visible research output.
- Added mock-fast assertions for custom download fallback suppression, platform release asset matching, and raw download URL redaction.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not create an installer, auto-updater, live browser adapter, custom download hook, or local binary override. It makes the future readiness lane auditable and keeps heavy dependency work out of ordinary research.

## Assimilation Wave 23: Level 5 Python Geo Lifecycle Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `cloakbrowser/geoip.py` as the syntax-level source of Python GeoIP optional dependency, proxy exit-IP lookup, country-to-locale mapping, GeoIP DB download/cache/update, and nonfatal failure behavior.
- Preserved the useful primitive: geo enrichment is a capability/readiness concern, not a default web-research behavior.
- Added contract metadata for optional GeoIP dependency state, no first-use GeoIP DB download during ordinary research, source admission, large-artifact lifecycle, atomic temp-file replacement, background refresh prohibition during research, policy-owned exit-IP echo provider order, raw proxy-host IP redaction, SOCKS dependency failure handling, and nonfatal geo resolution.
- Kept raw exit IPs, proxy host IPs, GeoIP DB paths, large DB downloads, and proxy behavior out of chat-visible research output.
- Added mock-fast assertions for no first-use GeoIP DB download during ordinary research, atomic DB replacement, and policy-owned exit-IP provider order.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable proxy, GeoIP lookup, WebRTC spoofing, GeoIP DB download, or live browser execution. It tightens the future capability contract so those behaviors cannot appear as ambient research side effects.

## Assimilation Wave 24: Level 5 CDP Service Pool Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `bin/cloakserve` as the syntax-level source of CloakBrowser's CDP multiplexer, per-seed process pool, local port allocation, connection refcounting, debugger URL rewrite, service data-dir cleanup, and host binding behavior.
- Preserved the useful primitive: service pooling can amortize browser launch cost later, but it is a separate Gateway-admitted capability, not ordinary web research execution.
- Added contract metadata for raw CDP denial, raw debugger port redaction, Gateway admission before public binding, policy-owned host binding, per-session identity seeds, seed validation and reserved seed blocklist, per-seed locking/process isolation, refcounts, localhost CDP allocation, bounded readiness polling, first-launch-wins session profile behavior, query/profile override denial, workflow passthrough arg denial, telemetry-only URL rewrite, and child-process shutdown.
- Kept service internals, ports, debugger URLs, raw CDP authority, fingerprint seed controls, passthrough browser args, and persistent session handles out of chat-visible research output.
- Added mock-fast assertions for service pool source pattern, raw CDP denial, data-dir-confined cleanup, and workflow passthrough-arg denial.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable a service adapter, raw CDP, persistent sessions, proxy behavior, fingerprint seed controls, public debug ports, or live browser execution. It only records the safety contract a future service/pool provider must satisfy.

## Assimilation Wave 25: Level 5 CDP Service Pool Test Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `tests/test_cloakserve.py` as the syntax-level source of CloakBrowser's service-pool query parsing, CLI parsing, URL rewriting, refcounting, seed validation, and cleanup containment invariants.
- Preserved the useful primitive: service tests define admission requirements for a future service provider; they do not justify exposing CDP/query/profile controls to ordinary research workflows.
- Added contract metadata for workflow denial of generic fingerprint query params, explicit repeated-query policy, policy-owned service CLI/data-dir/headless/debug flags, remote-debugging flag stripping, workflow passthrough denial, and Gateway-owned WebSocket scheme resolution.
- Confirmed previously integrated service invariants: seed validation, reserved seed blocklist, connection refcounting, data-dir-confined cleanup, raw CDP redaction, and service-pool default-off posture.
- Added mock-fast assertions for generic fingerprint query denial, remote-debugging passthrough stripping, and policy-owned service data dir.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable service mode, raw CDP, query-profile overrides, public debug ports, persistent sessions, proxy behavior, or live browser execution. It tightens the future service-provider admission checklist.

## Assimilation Wave 26: Level 5 Launch Test Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `tests/test_launch.py` as the syntax-level source of basic launch, close, page navigation, sync/async launch parity, binary-info, extra-args, and browser-surface probe invariants.
- Preserved the useful primitive: a future browser adapter must prove launch, connection, page navigation, and cleanup as internal runtime facts before any page content can be promoted.
- Quarantined non-portable anti-detection-style checks by treating fingerprint/browser-surface probe results as telemetry only; they do not become ordinary research evidence and do not justify stealth behavior by default.
- Added launch execution contract metadata for admitted-adapter launch, connected-browser proof, close-after-capture, sync/async parity, page-title candidate metadata, page-navigation packaging requirements, raw handle redaction, binary-info telemetry, and separate capability requirements for stealth patches.
- Added mock-fast assertions that the launch contract is visible in browser materialization diagnostics without enabling live browser execution.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable live browser execution, stealth patching, proxy/session behavior, arbitrary launch args, or treating browser-surface probes as user-facing evidence.

## Assimilation Wave 27: Level 5 Context Option Lane Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `tests/test_launch_context.py` as the syntax-level source of context option forwarding, default viewport behavior, locale/timezone lane separation, GeoIP fill behavior, sync/async close cleanup, and async cancellation cleanup.
- Preserved the useful primitive: context setup is a strict lane-separation problem, where ordinary workflows cannot pass raw context kwargs, storage state, viewport, user agent, color scheme, locale, timezone, proxy, or GeoIP knobs.
- Added context contract metadata for policy-owned default viewport, denial of caller viewport/user-agent/color-scheme controls, locale/timezone CDP-emulation denial, binary profile field ownership, proxy-gated GeoIP profile fills, generic workflow context kwarg denial, and storage-state session capability requirements.
- Kept cleanup invariants aligned with prior wrapper lifecycle work: context creation failure and async cancellation must close the browser before any future live adapter is admitted.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable raw Playwright context option passthrough, persistent storage state, GeoIP/proxy behavior, or locale/timezone spoofing from user/workflow request fields.

## Assimilation Wave 28: Level 5 Argument Compiler Test Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `tests/test_build_args.py` as the syntax-level source of timezone/locale injection, alias resolution, argument deduplication, dedicated-field precedence, non-value flag preservation, debug override logging, and WebRTC IP flag resolution.
- Preserved the useful primitive: browser launch arguments must compile from policy/profile state into one effective flag per key before adapter launch.
- Kept unsafe portability boundaries intact: ordinary workflows cannot pass raw fingerprint args, raw WebRTC IP args, arbitrary non-value flags, or hidden browser args.
- Added argument compiler metadata for single-effective-flag enforcement, dedicated locale/timezone precedence, policy admission for non-value flags, timezone alias consumption before context kwargs, admitted proxy exit-IP requirements for WebRTC auto resolution, unresolved-auto removal, and raw WebRTC IP redaction.
- Kept override/debug traces telemetry-only.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable raw browser argument passthrough, fingerprint spoofing, WebRTC IP spoofing, proxy use, or live browser execution.

## Assimilation Wave 29: Level 5 Backend Resolution Review

Status: reviewed; no new code required.

Reviewed:

- Parsed `tests/test_backend.py` as the syntax-level source of backend defaulting, explicit backend selection, environment fallback, parameter-over-env precedence, and invalid backend failure.
- Confirmed this file does not add a new primitive beyond previous adapter parity and wrapper lifecycle passes.
- Kept the relevant Infring rule unchanged: backend selection is policy/operator-owned, ordinary workflows cannot select a browser backend, invalid backend config must fail closed, and no backend-specific path may bypass the shared argument, proxy, geo, cleanup, evidence, or redaction contracts.

Validation:

- `git diff --check -- docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Important boundary:

This wave does not enable Playwright/Patchright/Puppeteer selection, environment-driven backend switching for user-facing workflows, or any live browser backend.

## Assimilation Wave 30: Level 5 Config Test Contract

Status: integrated and focused-tested.

Implemented:

- Parsed `tests/test_config.py` as the syntax-level source of platform-specific binary paths, archive extension/name construction, fallback download URL shape, cache directory overrides, unsupported platform handling, and default stealth/fingerprint args.
- Preserved the useful primitive: platform and dependency paths are runtime/operator readiness facts, not workflow/user-facing research controls.
- Added default config metadata for policy-owned platform binary path templates, archive extension/name ownership, fallback download URLs as operator-only readiness behavior, cache dir env override boundaries, unsupported-platform fail-closed behavior, random seed generation as operator-profile-only behavior, GPU fingerprint flag quarantine, and cache-dir redaction.
- Kept previous boundaries intact: ordinary research does not install/download, switch platform, spoof fingerprint state, or expose raw cache/download paths.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave does not enable surprise dependency installation, custom download URLs, cache directory overrides, platform spoofing, random fingerprint generation, or GPU fingerprint control during ordinary research.

## Assimilation Wave 31: Level 5 Proxy Test Contract

Status: integrated into future proxy capability contract; capability remains deferred.

Implemented:

- Parsed `tests/test_proxy.py` as the syntax-level source of proxy URL parsing, credential extraction, bare proxy normalization, SOCKS/SOCKS5H detection, adapter arg lane selection, credential encoding/idempotence, redacted notices, malformed proxy handling, IPv6 bracket preservation, and GeoIP proxy-server extraction.
- Preserved the useful primitive: proxy data is structured, secret-bearing capability input, never an ordinary research request field.
- Added proxy contract metadata for schemeless normalization after admission, credential removal from server URLs, username-only credentials, SOCKS5H support, idempotent SOCKS credential encoding, redacted encoding notices, malformed/nonstandard SOCKS rejection before adapter execution, IPv6 bracket preservation, and port-zero proxy policy admission.
- Kept the Infring behavior stricter than CloakBrowser where appropriate: malformed/nonstandard proxy shapes should fail closed before adapter execution instead of being passed through to Chromium.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable proxy use, proxy rotation, proxy credentials in workflow requests, GeoIP lookup, WebRTC spoofing, or live browser execution.

## Assimilation Wave 32: Level 5 GeoIP Test Contract

Status: integrated into future geo/proxy capability contract; capability remains deferred.

Implemented:

- Parsed `tests/test_geoip.py` as the syntax-level source of proxy IP extraction, private IP detection, country-locale map shape, missing dependency behavior, DB-missing behavior, explicit timezone/locale precedence, exit-IP consistency, and timeout handling.
- Preserved the useful primitive: GeoIP is capability metadata for a future admitted proxy/browser lane, not a source of research truth and not an ordinary workflow knob.
- Added geo consistency metadata for literal proxy IP extraction as telemetry only, invalid proxy URL nonfatal handling, BCP47 country-locale map requirements, fill-only-missing locale/timezone behavior, explicit profile precedence, exit-IP consistency even when profile fields are complete, timeout preservation of existing profile fields, and private IP evidence quarantine.
- Kept optional dependency and GeoIP DB behavior fail-closed/degraded without blocking ordinary research or downloading large artifacts during a user research turn.

Validation:

- `cargo fmt --check`
- `jq empty core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL5_SYNTAX_IMPLEMENTATION_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `cargo test -p infring-ops-core browser_materialization --lib`

Important boundary:

This wave still does not enable proxy use, GeoIP lookup, GeoIP DB download, WebRTC spoofing, private IP evidence, profile spoofing, or live browser execution.
