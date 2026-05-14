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
| 1 | `examples/integrations/aws_lambda/lambda_handler.py` | integrated: boundary contract pass 001 | What is the smallest safe one-shot navigate/wait/capture/close loop? | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` plus future adapter part. |
| 2 | `tests/test_lambda_security.py` | integrated: safety test pass 002 | Which security invariants must be locked before live browser execution? | Browser materialization contract tests. |
| 3 | `js/src/playwright.ts` | integrated: context boundary pass 003 | What launch/context cleanup and option filtering shape should the adapter mimic? | Future local browser adapter helper. |
| 4 | `js/src/types.ts` | integrated: API surface pass 004 | Which request/profile fields are real API surface versus convenience wrappers? | Tool CD/policy schema audit. |
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

## File Pass 001: `examples/integrations/aws_lambda/lambda_handler.py`

Status: `integrated: boundary contract`

Source lines inspected: 1-354

This file is the cleanest CloakBrowser source for the first Infring browser-materialization primitive because it is a one-shot endpoint, not a full browser session manager. The useful pattern is not Lambda-specific syntax; it is the security and lifecycle shape around a single browser-backed page materialization.

### Extracted Syntax Patterns

| Source Lines | Pattern | Infring Mapping | Decision |
| --- | --- | --- | --- |
| 79-97 | Parse URL, allow only HTTP(S), require hostname, resolve DNS, reject any non-global IP. | Reuse the existing fetch SSRF guard before browser execution; the live adapter must also re-run final URL validation after navigation. | Accept. |
| 126-148, 317-329 | Build launch kwargs internally and strip caller `extra_args` / `_strategy_args` before retry execution. | Caller-supplied browser args, launch flags, CDP controls, and hidden strategy fields stay denied at the API boundary. Internal profile compilation remains policy-owned. | Accept. |
| 151-179 | Smart wait polls `document.documentElement.outerHTML.length` until stable or bounded by `max_settle_ms`. | Add a policy-owned page readiness strategy later. Do not expose raw JS or arbitrary wait scripts to workflow/tool callers. | Accept as bounded readiness pattern. |
| 181-208 | Smart wait is default unless explicit bounded wait fields are provided. | Keep default wait strategy in Tool CD/profile policy; allow only bounded selector/load/fixed waits if admitted. | Accept. |
| 211-233 | Launch retry isolates Xvfb/browser startup races from retrieval quality. | If live adapter startup exists, classify adapter readiness failures separately from page/evidence quality and keep retry telemetry internal. | Accept as diagnostic split, not hidden broad retry. |
| 236-261 | Classify cert and timeout errors into bounded strategy overrides; leave DNS/SSL/refused as terminal. | Convert into provider retry recommendations and blocker diagnostics. Certificate bypass is not admitted by default and should remain a future explicit policy decision. | Partially accept. |
| 264-303 | Launch context, navigate with bounded timeout, validate final URL, wait, validate final URL again, capture title/final URL/html/screenshot, always close context. | This is the target single-shot adapter loop. Raw HTML/screenshot stay artifact refs; visible evidence gets title, final URL, extracted text/markdown, links, blocker class, confidence, and cleanup status. | Accept. |
| 306-314, 341-350 | Failure history is recorded with strategy and error snippets, then surfaced in diagnostics. | Keep retry history telemetry-only; final chat should only see synthesized high-level limitation if relevant. | Accept with projection boundary. |

### Concrete Integration Targets

| Target | Required Change | Notes |
| --- | --- | --- |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` | Extend the materialization contract so response metadata can represent pre-navigation URL safety, final URL safety, page readiness strategy, cleanup status, and retry recommendation. | Integrated as default-off boundary diagnostics; no live browser execution added. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` tests | Add fixture tests for denied caller controls, pre/final URL safety projection, readiness strategy projection, raw-artifact quarantine, and cleanup status. | Integrated in contract tests without live browser access. |
| `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Confirm the Tool CD names the allowed readiness fields and output metadata without allowing raw launch args, raw scripts, proxy fields, or session handles. | Integrated by adding safety/readiness/cleanup/retry fields to the materialized-page output contract. |
| `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md` | Mark this file pass complete and queue the next security-test pass. | Integrated; next pass remains `tests/test_lambda_security.py`. |

### Rejected Or Deferred From This File

| Source Feature | Decision | Reason |
| --- | --- | --- |
| Headed/Xvfb Lambda details | Reject for current primitive | Runtime-specific container mechanics do not belong in provider-neutral Tool CD behavior. |
| Caller proxy, humanize, geoip, locale, timezone, viewport, user-agent fields | Defer | Some may be useful later, but they require explicit capability admission and should not enter the first read-only materialization loop. |
| `--ignore-certificate-errors` retry strategy | Defer behind policy | It can retrieve more pages, but it weakens trust and must not become an invisible default. |
| Screenshot bytes in direct response | Reject for chat-visible paths | Screenshots, like raw HTML, must become artifact refs with bounded preview metadata. |
| Embedded full diagnostic snapshot in user-facing error | Reject | Xvfb logs, env details, retry traces, and process state are telemetry-only in Infring. |

### Pass 001 Outcome

The first implementation slice did not attempt full CloakBrowser parity. It added the smallest testable browser materialization boundary contract:

```text
safe input URL
-> admitted/default-off provider boundary
-> internally compiled profile/readiness strategy
-> one bounded navigation contract
-> final URL safety recheck contract
-> title/text/link extraction metadata contract
-> raw artifact refs only
-> blocker/cleanup/retry diagnostics
-> evidence candidate handoff
```

Validation: `cargo test -p infring-ops-core browser_materialization --lib` passed 6/6 targeted tests.

The immediate next source file is `tests/test_lambda_security.py`, because it should tell us which of these safety invariants CloakBrowser considered non-negotiable and which edge cases need Infring contract tests.

## File Pass 002: `tests/test_lambda_security.py`

Status: `integrated: safety tests`

Source lines inspected: 1-171

This file is useful because it turns the Lambda handler pattern into concrete security invariants. It is not about Python or Lambda; it is about proving that browser materialization never becomes a wider attack surface than fetch.

### Extracted Syntax Patterns

| Source Lines | Pattern | Infring Mapping | Decision |
| --- | --- | --- | --- |
| 18-47 | Reject non-HTTP(S) schemes, missing hostnames, and accept HTTP(S) case-insensitively. | Browser materialization now has tests for non-HTTP scheme rejection and uppercase HTTP parsing. A shared URL guard bug was fixed so authority parsing matches case-insensitive scheme detection. | Integrated. |
| 49-80 | Block metadata, loopback, localhost, private ranges, unspecified IPs, CGNAT, IPv6 loopback, and IPv4-mapped IPv6. | Browser materialization tests now cover the same restricted target classes before adapter execution. | Integrated. |
| 82-112 | Ignore caller `extra_args`, keep internal strategy args private, and preserve only runtime-owned hardening flags. | Browser materialization now denies `extra_args` and `_strategy_args` at the request boundary; strategy args remain internal telemetry/retry concepts only. | Integrated. |
| 114-130 | Treat post-navigation redirected targets as safety-critical. | Boundary already projects final URL revalidation as mandatory and not observed until adapter execution; tests assert this contract. | Integrated from pass 001, confirmed here. |
| 131-171 | Validate final URL before content/result construction. | Boundary contract says capture happens only after final URL safety; live adapter must enforce this before creating raw artifacts. | Integrated as contract; live adapter still pending. |

### Concrete Integration Completed

| Target | Change |
| --- | --- |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs` | Fixed case-insensitive HTTP(S) authority parsing so uppercase schemes do not corrupt host extraction. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` | Added `extra_args` and `_strategy_args` to denied caller controls. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | Added the same denied fields to default policy. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs` | Added the same denied fields to runtime profile-compilation metadata. |
| `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Added the same denied fields to the Tool CD request contract. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/020-fetch-policy-and-provider-contract-tests.rs` | Added a shared SSRF guard test for uppercase HTTP scheme authority parsing. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs` | Added tests for non-HTTP scheme rejection, internal/private targets, uppercase HTTP, and denied `extra_args` / `_strategy_args`. |

### Pass 002 Outcome

The browser materialization boundary now locks the same security invariants CloakBrowser considered non-negotiable, while keeping execution default-off:

```text
reject non-web schemes
-> reject missing/unsafe host targets
-> reject credentials and internal network destinations
-> reject caller-controlled launch/strategy args
-> require final URL revalidation before capture
-> keep retry/strategy details telemetry-only
```

Validation: `cargo test -p infring-ops-core browser_materialization --lib` passed 10/10 targeted tests.

The immediate next source file is `js/src/playwright.ts`, which should tell us how much launch/context cleanup and option filtering needs to become an adapter-helper contract before we build any live browser provider.

## File Pass 003: `js/src/playwright.ts`

Status: `integrated: context boundary`

Source lines inspected: 1-235

This file is the best CloakBrowser source for launch/context ownership. The useful pattern is that callers get a simple high-level API, while the adapter owns binary selection, launch args, context creation, context conflict handling, cleanup, and optional capability hooks. For Infring, that maps to a stricter rule: browser profile and context options are policy-owned until a future capability explicitly admits more.

### Extracted Syntax Patterns

| Source Lines | Pattern | Infring Mapping | Decision |
| --- | --- | --- | --- |
| 14-22 | Accept `timezoneId` as an alias but normalize to one profile field. | Future profile compiler may normalize aliases internally; user requests cannot pass direct timezone fields in the current materialization request. | Accept as policy compiler pattern, deny at current caller boundary. |
| 24-45 | Strip `locale` and `timezoneId` from raw Playwright context options because they use detectable CDP emulation. | Add context-conflict metadata and deny caller `contextOptions`; adapter may normalize conflicts internally later. | Integrated. |
| 60-79 | Adapter owns binary path, geo/proxy resolution, WebRTC args, compiled launch args, and ignored defaults. | Keep binary/proxy/geo/launch args out of request authority; represent them as profile/readiness metadata only. | Integrated as denied controls and profile contract. |
| 81-90, 149-158, 216-225 | Human-like patching is opt-in at launch/context level. | Defer; humanized interaction is a separate future capability, not part of read-only materialization. | Deferred and explicitly denied at current boundary. |
| 111-147 | `launchContext` creates browser first, closes browser if context creation fails, and patches `context.close()` to close the browser. | Add context/cleanup contract fields: close browser on context-creation failure and context close closes browser. | Integrated as boundary contract. |
| 184-228 | Persistent context is a separate launch path with user data dir and session retention. | Defer; persistent sessions require retention, identity, TTL, and cleanup authority. | Deferred and explicitly denied at current boundary. |
| 234-235 | Test-only arg compiler export. | Infring should prove profile compilation through tests, not expose raw compiler controls to workflow callers. | Accept as testing pattern. |

### Concrete Integration Completed

| Target | Change |
| --- | --- |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` | Added a context contract projection and denied direct Playwright/profile override fields such as `contextOptions`, `launchOptions`, `headless`, `viewport`, `locale`, `timezoneId`, `humanize`, and `geoip`. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | Added the same denied request fields and profile lifecycle fields to default policy. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs` | Projected context-conflict fields, caller-context-option denial, cleanup obligations, and deferred human/session behavior through runtime metadata. |
| `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Added Tool CD profile-contract fields for context option denial, context conflict fields, cleanup obligations, and deferred persistent/human behavior. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs` | Added tests proving context contract projection and rejection of direct Playwright/profile overrides. |

### Pass 003 Outcome

The boundary now treats the browser context as an adapter-owned resource:

```text
workflow/tool request
-> no raw Playwright context or launch options
-> no caller profile overrides
-> profile/context conflicts are policy metadata
-> adapter must close browser if context creation fails
-> context close must close browser
-> persistent session and humanized interaction remain separate future capabilities
```

Validation: `cargo test -p infring-ops-core browser_materialization --lib` passed 11/11 targeted tests.

The next source file is `js/src/types.ts`, which should be used to audit the public API surface and decide which fields remain denied, deferred, or promoted into Tool CD request/profile schema.

## File Pass 004: `js/src/types.ts`

Status: `integrated: API surface audit`

Source lines inspected: 1-72

This file is useful because it names the public knobs CloakBrowser lets callers pass into browser launch, context launch, persistent context launch, and binary readiness surfaces. For Infring, the pattern is not "copy these fields into the user request." The pattern is to make the request boundary explicit about which browser controls are caller-owned, policy-owned, deferred, or never chat-visible.

### Extracted Syntax Patterns

| Source Lines | Pattern | Infring Mapping | Decision |
| --- | --- | --- | --- |
| 8-36 | `LaunchOptions` exposes headless mode, proxy, raw Chromium args, stealth-arg toggle, timezone, locale, geoip, raw launch options, and humanization controls. | These are profile/capability controls, not ordinary materialization request fields. The current primitive rejects the direct aliases and keeps profile compilation policy-owned. | Integrated as denied caller fields. |
| 38-58 | `LaunchContextOptions` adds user-agent, viewport, timezone alias, color scheme, and raw `contextOptions` including storage state, permissions, geolocation, headers, and credentials. | Context conflict and raw context fields stay adapter-owned until separately admitted. Direct caller context overrides are denied before URL safety or adapter readiness. | Integrated as denied caller fields and context contract. |
| 60-63 | Persistent context requires `userDataDir`. | Persistent profile/session retention is a separate future capability with TTL, cleanup, and identity rules. It is not part of stateless browser materialization. | Integrated as denied caller field. |
| 65-72 | `BinaryInfo` exposes version, platform, path, installed state, cache dir, and download URL. | Readiness lifecycle may project installed/version/status, but raw binary paths/download URLs should remain provider diagnostics rather than user-facing evidence. | Accepted as readiness metadata pattern; no new user request field. |

### Concrete Integration Completed

| Target | Change |
| --- | --- |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs` | Expanded denied request fields to include CloakBrowser's public API aliases: `args`, `stealthArgs`, camelCase proxy/session/storage/local-file aliases, and `userDataDir`. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | Added the same API-surface denial list to default policy so the CD/player boundary stays visible. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs` | Projected the same denied API surface through runtime profile-compilation metadata. |
| `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Added the same API-surface fields to the Tool CD request contract. |
| `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs` | Extended mock-fast rejection tests for raw args, stealth arg toggles, persistent user data dirs, and storage state. |

### Rejected Or Deferred From This File

| Source Feature | Decision | Reason |
| --- | --- | --- |
| Direct caller `args` / `stealthArgs` | Reject for current primitive | Launch flags are policy/profile compiler output, not request authority. |
| Direct caller `proxy` / proxy credentials | Defer | Proxy use requires a separate permission and secret-redaction lane. |
| Direct caller `storageState` / `userDataDir` | Defer | Persistent state changes identity, retention, cleanup, and privacy obligations. |
| Direct caller user-agent, viewport, locale, timezone, color scheme | Defer | These can be profile fields later, but the first primitive should prove stateless materialization before admitting fingerprint/profile overrides. |
| User-visible raw binary path/cache/download URL | Reject | Binary lifecycle is runtime/provider readiness, not research evidence. |

### Pass 004 Outcome

The browser materialization request is now aligned against CloakBrowser's public API surface:

```text
URL + admission handle + bounded extraction/readiness fields
-> deny direct launch/context/profile/session/proxy controls
-> project profile/readiness metadata internally
-> keep future profile/session/proxy capabilities explicit
```

Validation target: `cargo test -p infring-ops-core browser_materialization --lib`.

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
