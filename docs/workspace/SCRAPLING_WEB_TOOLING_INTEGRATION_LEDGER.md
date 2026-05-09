# Scrapling Web Tooling Integration Ledger

Created: 2026-05-08

Source repo: `https://github.com/D4Vinci/Scrapling`

Local source clone: `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling`

External assimilation baseline: `/Users/jay/.openclaw/workspace/local/workspace/shadow/scrapling-assimilation/ASSIMILATION_LEDGER.md`

## Goal

Track the repo-local implementation pass that turns already-assimilated Scrapling patterns into live web-tool behavior. This ledger is intentionally about patterns and target behavior, not copied source or syntax.

## Guardrails

- Prefer policy/CD-controlled behavior over Rust-specific application logic.
- Do not add provider-specific source lists or domain-specific search routes.
- Do not add hardcoded response formats.
- Keep explicit agent-submitted query packs authoritative.
- Expose recovery behavior in tool artifacts instead of hiding it from the agent.

## Parsed This Pass

| Path | Status | Relevant pattern | Result |
| --- | --- | --- | --- |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/scrapling-assimilation/ASSIMILATION_LEDGER.md` | parsed | prior pattern inventory | Isolated the remaining gap as live retrieval recovery, not more Tool CD schema work. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/client/runtime/config/batch_query_policy.json` | parsed + updated | CD/policy-owned retrieval behavior | Added policy-visible broad-current query recovery templates. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/010-core_parts/000-part.rs` | parsed + updated | default CD fallback | Mirrored the policy defaults used when no repo policy file exists. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs` | parsed + updated | current-query classification | Treated the active year as current intent without enumerating month strings. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/018-request-and-cache.rs` | parsed + updated | visible query-plan recovery | Added policy-driven recovery query planning while preserving explicit query-pack authority. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/040-api-batch-query_parts/000-combined.rs` | parsed + updated | live batch query execution path | Passed loaded policy into query-plan resolution. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/041-quality-tests.rs` | parsed + updated | regression proof | Added a broad-current recovery test with visible query-plan assertions. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/042-cache-rewrite-tests.rs` | parsed + updated | explicit query-pack preservation | Updated test call sites for policy-aware query-plan resolution. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs` | parsed + updated | current-intent proof | Added current-year intent coverage. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/spiders/architecture.md` | parsed | scheduler/engine separation | Confirmed scheduling, execution, and result collection are separate concerns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/spiders/advanced.md` | parsed | bounded retries and priorities | Reinforced that retry/fanout policy should be explicit and bounded. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/spiders/proxy-blocking.md` | parsed | blocked/failed/soft-result distinction | Extracted the general distinction between blocked/failed transport and a low-quality page result. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/spiders/sessions.md` | parsed | session lifecycle boundaries | Reinforced explicit session ownership and observable routing rather than hidden runtime behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/scheduler.py` | parsed | priority queue, dedupe, retry bypass | Extracted request fingerprinting and `dont_filter` as controlled replay concepts. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/engine.py` | parsed | execution stats and blocked retry handling | Extracted request-quality lanes, retry budgets, and stats as tool-result metadata patterns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/request.py` | parsed | request fingerprint scope | Reinforced stable request identity without domain-specific prompt logic. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/session.py` | parsed | explicit session manager | Reinforced explicit/lazy session lifecycle signals already reflected in Tool CD session fields. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/cache.py` | parsed | response-cache receipt preservation | Reinforced cache hit/miss as an artifact-level diagnostic, not synthesis text. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/spider.py` | parsed | blocked response hook and stats counters | Confirmed block detection should feed quality lanes and stats, not provider outage state by default. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/result.py` | parsed | crawl stats/result contract | Extracted counters as useful low-level evidence for tool boundary diagnostics. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/checkpoint.py` | parsed | resumable queue snapshots | Logged as lower-priority future pattern for long retrieval jobs, not current live-search work. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_scheduler.py` | parsed | dedupe/retry behavior proof | Used as validation shape for soft-vs-hard request handling. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_engine.py` | parsed | blocked retry, stats, offsite filtering | Used to separate content-quality misses from transport/provider failures. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_cache.py` | parsed | cache hit/miss semantics | Confirmed stale generated caches can mask live-retrieval behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_request.py` | parsed | fingerprint and retry metadata | Reinforced stable request identity and explicit retry metadata. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_result.py` | parsed | result counters | Reinforced exposing counts as diagnostics. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_session.py` | parsed | eager/lazy session lifecycle | Reinforced Tool CD session lifecycle fields. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_spider.py` | parsed | block-code defaults and session config errors | Used to validate that blocked/error classes need different lanes. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_robotstxt.py` | parsed | domain-level cache/policy separation | Logged for future permission-policy work; not assimilated into search behavior now. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/spiders/test_checkpoint.py` | parsed | atomic resumable state | Logged for future long-run retrieval jobs. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/parsing/main_classes.md` | parsed | lazy content extraction and ignored tags | Reinforced that extraction should preserve clean visible text while quarantining raw/noisy payloads. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/parsing/selection.md` | parsed | optional selector/text/regex narrowing | Kept as optional evidence-narrowing capability, not a required workflow shape. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/parsing/adaptive.md` | parsed | save/match relocation split | Logged as a future pattern for resilient source-specific extraction, not current broad-search routing. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/docs/development/adaptive_storage_system.md` | parsed | domain-scoped adaptive state | Reinforced that persistent source-learning should be scoped and explicit, not hidden global behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/fetchers/stealth_chrome.py` | parsed | high-cost escalation surface | Extracted only declared capability/readiness/session signals; bypass/challenge-solving behavior remains out of scope. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/engines/_browsers/_base.py` | parsed | page pool, readiness, XHR capture boundary | Reinforced explicit session/page-pool stats and readiness fields already reflected in Tool CD surfaces. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/engines/_browsers/_stealth.py` | parsed for boundary only | challenge-detection/escalation surface | No mechanics assimilated; only confirms blocked/challenge should be a quality lane. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/engines/toolbelt/proxy_rotation.py` | parsed | pluggable rotation strategy and proxy-error detection | Logged as permissioned future provider-capability work; no proxy behavior imported. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/engines/toolbelt/ad_domains.py` | boundary scanned | resource blocking list | Explicitly not copied; only the policy-knob pattern remains useful. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/scrapling/spiders/robotstxt.py` | parsed | per-domain permission cache and delay hints | Logged for future permission-policy layer, not current search behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/fetchers/test_proxy_rotation.py` | parsed | strategy validation and thread-safe copy semantics | Reinforced that provider rotation must be declared and observable, not implicit. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/fetchers/sync/test_stealth_session.py` | parsed for boundary only | challenge detection and high-cost session settings | No challenge-solving tactics imported. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/parser/test_general.py` | parsed | selector, text, regex, traversal, error handling | Reinforced extraction as a clean content operation with failure surfaces. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/parser/test_find_similar_advanced.py` | parsed | structural similarity with thresholds | Logged as a future source-learning primitive; not useful for immediate broad web search. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/parser/test_selectors_filter.py` | parsed | chainable filters and empty-result behavior | Reinforced empty-result behavior should stay structured and non-exceptional. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/Scrapling/tests/parser/test_adaptive.py` | parsed | selector relocation after structure change | Logged for future targeted extraction resilience. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/client/runtime/config/web_conduit_policy.json` | parsed + updated | provider health policy | Added policy-owned soft failure classes. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | parsed + updated | default provider health policy | Mirrored soft failure classes for no-file fallback. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_provider_runtime_parts/010-provider-chain-and-health.rs` | parsed + updated | provider health recording | Soft search-quality failures no longer open global provider circuits. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_provider_runtime_parts/017-provider-public-contracts.rs` | parsed + updated | diagnostics exposure | Provider health snapshots now expose `last_failure_class`. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_provider_runtime_parts/020-cache-and-tests.rs` | parsed + updated | regression proof | Added a focused soft-failure circuit regression. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/030-serper-bing-and-fetch.rs` | parsed + updated | rejected payload quality boundary | Search payloads that fail result-quality selection no longer report as successful evidence. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/020-domain-and-render.rs` | parsed + updated | provider URL construction boundary | Added a keyless current/news document API URL builder without changing agent prompt routing. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/050-search-providers_parts/002-segment.rs` | parsed + updated | provider execution boundary | Added the GDELT Doc API provider adapter as a normal search provider, not a special-case workflow. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/050-search-providers_parts/003-segment.rs` | parsed + updated | query/result alignment | Generalized alignment so related word forms can pass without allowing one-term off-topic payloads. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/060-search-orchestration_parts/001-segment_parts/670-api-search-parts/000-combined.rs` | parsed + updated | provider-chain selected-vs-last payload boundary | Last payload fallback is marked rejected when it did not pass query/result quality checks. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/060-search-orchestration_parts/001-segment_parts/670-api-search-parts/030-segment.rs` | parsed + updated | generated provider URL branch | Kept split segment in sync with the combined provider chain. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/060-search-orchestration_parts/001-segment_parts/670-api-search-parts/040-segment.rs` | parsed + updated | generated provider execution branch | Kept split segment in sync with provider execution and query-mismatch classification. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/060-search-orchestration_parts/001-segment_parts/670-api-search-parts/050-segment.rs` | parsed + updated | generated provider finalization branch | Kept split segment in sync with rejected-payload and failure-mode reporting. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/065-setup-and-migration.rs` | parsed + updated | provider setup surface | Exposed GDELT as a keyless current-events document-index fallback in setup/provider help. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined_parts/020-collect-candidates-from-stage-payload-to-retrieve-web-candidates-for.rs` | parsed + updated | transport success vs synthesis usability | Provider artifacts now expose `provider_transport_ok`, `result_quality`, and `synthesis_candidate_count`. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs` | parsed + updated | regression proof | Added low-relevance provider artifact quality test. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/live-eval-polling-fix-20260508/core/layer0/ops/src/web_conduit_parts/095-openclaw-search-tool-tests.rs` | parsed + updated | regression proof | Added rejected search payload, GDELT receipt rendering, and related-word alignment tests. |

## Implemented Slice

`SCRAPLING-LIVE-001`: policy-visible broad-current recovery.

Scrapling pattern source: retrieval ladder, result quality lanes, and scheduler/backoff concepts from `SCRAPLING-PATTERN-001`, `SCRAPLING-PATTERN-008`, and `SCRAPLING-PATTERN-012`.

Target behavior:

- A broad, current-ish research query can fan out into a small recovery query pack.
- The query pack is declared in `batch_query_policy.json`.
- The resulting `query_plan` and `query_plan_source` are returned in the batch-query artifact.
- Explicit `queries` from the agent still bypass policy recovery.
- The runtime avoids generating duplicate current-year queries such as `2026 2026`.
- Explicit agent-submitted query packs are bounded by candidate budget instead of evidence-output budget so source-targeted queries are not dropped before retrieval.

Current status: implemented, targeted validation passed.

Validation:

- `cargo test --manifest-path core/layer0/ops/Cargo.toml broad_current_research_query_uses_policy_visible_recovery_pack -- --nocapture`
- `cargo test --manifest-path core/layer0/ops/Cargo.toml query_plan -- --nocapture`
- `cargo test --manifest-path core/layer0/ops/Cargo.toml current_year_counts_as_current_web_intent -- --nocapture`
- Live smoke: `./target/debug/infring-ops batch-query query --source=web --query="scientific breakthroughs 2026" --aperture=medium`

Live-smoke result:

- Query planning worked: `query_plan_source=policy_broad_current_research_recovery`.
- Evidence did not improve yet: providers returned off-topic Bing RSS payloads, empty DuckDuckGo Instant payloads, and missing Serper credentials.
- Next bottleneck is provider/result quality and fallback coverage, not hidden workflow routing.

`SCRAPLING-LIVE-002`: policy-owned soft provider failure classes.

Scrapling pattern source: blocked/retry handling, stats separation, and scheduler retry behavior from the spider engine and tests. The important pattern is not the crawler implementation; it is the separation between a request/result-quality miss and a broken provider/session.

Target behavior:

- Low-signal, no-summary, and off-topic/query-mismatch provider outcomes are classified by policy as soft search-quality failures.
- Soft search-quality failures do not open the provider circuit or accumulate global provider outage counts.
- Hard transport/configuration failures still count toward provider circuit behavior.
- Provider diagnostics expose the class of the most recent failure so the agent/tool boundary is easier to reason about.
- The behavior is declared in `web_conduit_policy.json`, with Rust only validating/executing the policy.

Current status: implemented, targeted validation passed.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml soft_search_failures_do_not_open_provider_circuit -- --nocapture`
- Live smoke after clearing generated provider/cache state: `./target/debug/infring-ops batch-query query --source=web --query="scientific breakthroughs 2026" --aperture=medium`

Live-smoke result:

- `duckduckgo`, `duckduckgo_lite`, and `bing_rss` soft misses left `circuit_open_until=0`.
- `last_failure_class=soft_no_circuit` for low-signal/off-topic search-quality misses.
- `serperdev` missing credentials still reports `last_failure_class=circuit_counting`.
- The tool still returned `status=no_results` because the available providers did not produce usable evidence. That is now a retrieval/provider quality gap, not a self-inflicted circuit-health failure.

`SCRAPLING-LIVE-003`: provider artifact dedupe before synthesis handoff.

Scrapling pattern source: scheduler fingerprint dedupe and result stats from the spider scheduler/engine layer. The extracted pattern is stable artifact identity, not crawler control flow.

Target behavior:

- Repeated provider artifacts with identical normalized error or content identity collapse before they reach the batch-query output.
- Partial failure details remain query-specific, so diagnostics still explain which query variants failed.
- The output and receipt expose `provider_result_count` and `provider_result_dedup_count`.
- This reduces duplicated low-quality provider context without changing tool routing, retrieval policy, or final answer format.

Current status: implemented, targeted validation passed.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml provider_result_dedup_collapses_repeated_content_and_errors -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml broad_current_query_drops_off_topic_provider_results_before_evidence -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops`
- Live smoke after clearing generated provider/cache state: `./target/debug/infring-ops batch-query query --source=web --query="scientific breakthroughs 2026" --aperture=medium`

Live-smoke result:

- `query_plan_source=policy_broad_current_research_recovery`.
- `provider_result_count=3`.
- `provider_result_dedup_count=6`.
- `status=no_results`, with no promoted evidence from off-topic provider payloads.

`SCRAPLING-LIVE-004`: rejected provider payloads cannot report search success.

Scrapling pattern source: result quality lanes and response receipt separation. The extracted pattern is that transport/provider success is not the same as usable evidence.

Target behavior:

- A provider payload that returned content but failed query/result quality is marked rejected.
- Rejected payloads retain source links as follow-up leads.
- Rejected payload text is kept as diagnostic context, not promoted as evidence.
- Search cache skips rejected payloads using the concrete failure mode.

Current status: implemented, targeted validation passed.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml rejected_search_payload_is_not_reported_as_success -- --nocapture`
- Direct live smoke: `./target/debug/infring-ops web-conduit search --query='scientific breakthroughs 2026 research news' --provider=bing --top-k=5 --cache-ttl-minutes=0 --summary-only=1`

Live-smoke result:

- `ok=false`
- `error=query_result_mismatch`
- `provider_payload_rejected=true`
- Links were retained for follow-up fetch attempts.

`SCRAPLING-LIVE-005`: provider result artifacts distinguish transport success from synthesis usability.

Scrapling pattern source: response receipt plus spider result stats. The extracted pattern is explicit result classification at the tool boundary.

Target behavior:

- Batch-query provider artifacts expose `provider_transport_ok`.
- Batch-query provider artifacts expose `result_quality`.
- Batch-query provider artifacts expose `synthesis_candidate_count`.
- A payload can be transport-successful while still `ok=false` for synthesis if it produced no eligible evidence.

Current status: implemented, targeted validation passed.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml provider_result_artifact_marks_low_relevance_payload_as_not_usable -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops`

Live-smoke result:

- Direct web-conduit rejection still works.
- Fresh batch-query smoke for `scientific breakthroughs 2026` hit provider timeouts in this run, so live proof of the new provider artifact fields remains pending on a non-timeout retrieval run.

`SCRAPLING-LIVE-006`: policy-declared current/news provider fallback.

Scrapling pattern source: provider capability boundaries, structured receipts, result-quality lanes, and provider-readiness diagnostics. The extracted pattern is better provider coverage and explicit result metadata, not browser/proxy/stealth mechanics.

Target behavior:

- Add a general keyless current/news document-index provider to the existing web-search provider chain.
- Keep the provider selected through provider policy and provider hints, not through domain-specific prompt wording.
- Return the same search receipt shape as other providers: summary, content, links, domains, raw count, filtered count, status/error.
- Preserve rate-limit and low-relevance conditions as tool artifacts so synthesis can distinguish provider availability from weak evidence.
- Avoid hardcoded science sources, response formats, or query templates.

Current status: implemented v0, targeted validation passed. Live provider availability is still rate-limit dependent.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml openclaw_search_tool_tests -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo build --manifest-path core/layer0/ops/Cargo.toml --bin infring-ops`

Live-smoke result:

- First direct `gdelt_doc` smoke for `scientific breakthroughs 2026` returned current/news links and source domains, but the old query/result alignment rejected the payload as `query_result_mismatch`.
- The alignment check now accepts related word forms generally, so `science/scientific` and singular/plural variants do not create false negatives when at least enough other intent terms match.
- Post-fix direct `gdelt_doc` smoke for `scientific breakthroughs 2026` returned `ok=true`, links, source domains, and no provider errors.
- The workflow-facing `batch-query` smoke still ended as `status=no_results` in this run because the auto provider chain hit low-signal DuckDuckGo, off-topic Bing, and a rate-limited GDELT attempt. That confirms the next gap is provider availability/backoff and batch-query evidence promotion, not the direct provider adapter.

`SCRAPLING-LIVE-007`: policy-gated provider recovery and stricter evidence promotion.

Scrapling pattern source: provider fallback ladders, result-quality lanes, and diagnostic/result separation. The extracted pattern is that weak provider output should trigger another admitted provider when policy allows it, while provider chrome and negative retrieval summaries stay diagnostic-only.

Target behavior:

- Keep recovery provider selection in the batch-query policy CD, not hardcoded to a user prompt, domain, or output format.
- Let `batch-query` ask a policy-declared recovery provider after the primary provider chain yields no synthesis-eligible candidates.
- Do not let synthetic titles such as `Web result from ...` create relevance overlap by themselves.
- Treat generic negative retrieval summaries such as `no usable search results` as low-signal tool chrome, not usable evidence.
- Preserve hard mismatch diagnostics, such as unrelated code/problem dumps, even when a broader no-results fallback would otherwise mask them.

Current status: implemented, targeted validation passed.

Validation:

- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml synthetic_web_result_prefix_does_not_create_relevance_overlap -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml policy_provider_recovery_promotes_usable_source_after_low_signal_chain -- --nocapture`
- `CARGO_INCREMENTAL=0 cargo test --manifest-path core/layer0/ops/Cargo.toml competitive_programming_dump_is_treated_as_query_mismatch_low_signal -- --nocapture`

Implementation note:

- This does not add a new domain-specific search path. The runtime only reads policy-declared recovery providers and applies general evidence-promotion rules.
- Full live workflow/golden measurement is still pending because this patch is a targeted upstream retrieval fix; the expected improvement should show up as fewer soft 6a failures when low-signal primary providers have a usable recovery source.

## Queued

- Re-run workflow/golden gates after the retrieval/provider changes are committed or otherwise isolated.
- Evaluate whether current/news provider rate-limit handling needs cache/backoff policy before becoming part of the default live eval path.
