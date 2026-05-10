# Firecrawl Web Tooling Assimilation Ledger

Created: 2026-05-10

Source repo: `https://github.com/firecrawl/firecrawl`

Local source clone: `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl`

## Goal

Track Firecrawl pattern assimilation for the web research tooling pivot from low-result recovery to high-volume candidate generation, filtering, extraction, and evidence packaging.

This ledger is intentionally about portable patterns, not copied source or provider-specific mechanics.

## Guardrails

- Prefer CD/policy-owned behavior over Rust application hardcoding.
- Do not add domain-specific research routes or fixed answer formats.
- Do not make Firecrawl the only web path; treat it as a source of patterns and an optional provider surface.
- Preserve explicit user/agent query intent instead of rewriting prompts into canned domains.
- Separate search candidate discovery, page extraction, result filtering, and final synthesis.
- Keep scraping, crawling, browser interaction, auth, and sensitive-page behavior behind existing permission boundaries.
- Track parsed source surfaces here so future passes can continue without repeating or missing coverage.

## Assimilation Targets

| ID | Target | Why it matters | Status |
| --- | --- | --- | --- |
| `FIRECRAWL-PATTERN-001` | High-recall search result buffer | Firecrawl asks for more candidates than final results need, then limits after ranking/filtering. This directly addresses tiny evidence pools. | active |
| `FIRECRAWL-PATTERN-002` | Search-plus-scrape evidence flow | Firecrawl search can attach scraped page content, not just SERP snippets. This is the likely next primitive for user-quality research. | active |
| `FIRECRAWL-PATTERN-003` | Multi-source result lanes | Search can return web/news/images lanes with separate result types. Our tool artifacts should keep lanes explicit. | active |
| `FIRECRAWL-PATTERN-004` | Category and domain filters as request policy | Firecrawl exposes categories like GitHub/research/PDF and include/exclude domains. If used, this belongs in CD/tool request contracts, not hardcoded gates. | active |
| `FIRECRAWL-PATTERN-005` | Async crawl/batch progress contract | Crawl/batch scrape returns status, total, completed, data, errors, and polling. Useful for long research jobs, not first-turn search. | active |
| `FIRECRAWL-PATTERN-006` | LLM-ready document contract | Document objects preserve markdown, summary, metadata, source URL, status, cache state, and errors. This maps well to evidence packs. | active |
| `FIRECRAWL-PATTERN-007` | Optional structured extraction | JSON/schema extraction is an optional layer after source retrieval, not a forced final answer shape. | pending |
| `FIRECRAWL-PATTERN-008` | Query context preservation | Firecrawl agent/search APIs accept an intent prompt and optional URLs. Our follow-up retries need the same idea: compile full research intent, not the latest utterance alone. | pending |

## Parsed This Pass

| Path | Status | Relevant pattern | Result |
| --- | --- | --- | --- |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/README.md` | parsed | endpoint taxonomy and LLM-ready data contract | Extracted search, scrape, agent, crawl, map, and batch scrape as separate primitives. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/search.ts` | parsed | search request lifecycle | Logged validated request, sources, categories, include/exclude domains, scrape options, and billing/telemetry as separable concerns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/execute.ts` | parsed | high-recall buffer and optional scrape after search | Extracted `limit * 2` candidate buffering and search-result scraping before final response. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/scrape.ts` | parsed | search results become scrape inputs | Extracted search-result-to-document expansion with source URL/title/description preserved. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/v2/index.ts` | parsed | provider chain fallback | Extracted provider chain shape: preferred engine, SearXNG fallback, DDG fallback, empty response if all fail. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/v2/searxng.ts` | parsed | paginated high-volume search | Extracted page-based candidate collection up to requested result count. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/v2/ddgsearch.ts` | partial | DDG pagination and anti-bot surface | Logged pagination/dedupe/anti-bot detection for later review; no bypass mechanics copied. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/search-query-builder.ts` | parsed | categories and include/exclude domains | Pattern is policy-declared search lanes; avoid copying hardcoded domain lists into Rust. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/python-sdk/firecrawl/v2/methods/search.py` | parsed | typed search response transformation | Extracted mixed result/document handling when search returns scraped content. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/python-sdk/firecrawl/v2/types.py` | partial | document/search/crawl/batch type contracts | Extracted document metadata and result lane models; crawl/batch/agent details still pending. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/js-sdk/firecrawl/src/v2/client.ts` | partial | SDK method boundaries | Confirmed search, scrape, map, crawl, batch, extract, agent, and watcher are separate client operations. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/scraper/scrapeURL/index.ts` | parsed | scrape orchestration boundary | Extracted immutable scrape metadata, feature flags, robots/permission checks, engine fallback, transforms, metadata/link extraction, and postprocessors as separable concerns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/scraper/scrapeURL/engines/index.ts` | parsed | capability-aware extraction engines | Extracted engine capability matrix, quality ranking, max reasonable times, force/lockdown constraints, and fallback selection by requested extraction features. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/scraper/scrapeURL/lib/removeUnwantedElements.ts` | parsed | LLM-ready content cleanup | Extracted main-content filtering, include/exclude tag policy, script/style stripping, link/image absolutization, and larger image selection as document-prep patterns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/scraper/scrapeURL/lib/extractMetadata.ts` | parsed | document metadata extraction | Extracted title, description, favicon, language, robots, OpenGraph, Dublin Core, article timestamps/tags, and custom metadata as evidence-pack enrichment fields. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/scraper/scrapeURL/lib/extractLinks.ts` | parsed | URL normalization and dedupe | Extracted base-href-aware link resolution, fragment filtering, mailto preservation, and unique absolute link output for follow-up discovery. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/crawl.ts` | parsed | async crawl kickoff and prompt-to-options boundary | Extracted explicit kickoff response with job ID/status URL, permissions, robots discovery, TTL, max concurrency, and prompt-generated options that do not overwrite explicitly supplied user options. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/batch-scrape.ts` | parsed | batch URL extraction workflow | Extracted URL validation, ignore-invalid behavior, append-to-existing-job, priority by batch size, queued scrape jobs, invalid URL reporting, and webhook start event. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/crawl-status.ts` | parsed | bounded polling result window | Extracted status/completed/total/credits/expires/data/next response shape, result byte cap, pagination cursor, robots warning, and “start higher in site” warning for tiny crawl result sets. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/crawl-errors.ts` | parsed | structured async errors | Extracted failed job listing with id/timestamp/url/code/error and robots-blocked side channel. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/map.ts` | parsed | site-map discovery primitive | Extracted site-constrained discovery with limit, search, sitemap, includeSubdomains, external-link policy, timeout/abort, and fallback resolver. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/map-utils.ts` | parsed | map discovery internals | Extracted index-plus-search-plus-sitemap aggregation, same-domain/subdomain/path filtering, query-parameter normalization, dedupe that preserves titles, cosine ranking for searched maps, and website-structure prompt context as optional intent support. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/controllers/v2/extract.ts` | parsed | optional structured extraction job | Extracted structured extraction as an async post-retrieval job with URL validation, job ID, source display options, and explicit ZDR/sensitive constraints. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/extract/fire-0/url-processor-f0.ts` | parsed | map-before-rerank extraction planning | Extracted prompt-preserving URL discovery, broader retry when a map returns only one URL, large candidate cap before reranking, and trace records for mapped/used/error URL states. Pattern is useful for high-volume candidate filtering, but should remain a general primitive rather than a fixed extraction route. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/extract/fire-0/reranker-f0.ts` | parsed | rank-before-scrape | Extracted chunked relevance ranking over URL/title/description rows with scores and reasons before spending extraction work. Pattern should map to policy-owned candidate scoring and optional LLM rerank later, not hardcoded domain behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/extract/build-prompts.ts` | parsed | intent-preserving query/rerank prompts | Extracted separation between SERP query phrasing, pre-rerank intent phrasing, and extraction instructions. Useful guidance: preserve intent and keep page content untrusted; do not force final answer format. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/extract/document-scraper.ts` | parsed | queued scrape document contract | Extracted queued page scrape with blocked URL check, trace timing/status/contentStats, single URL double-timeout retry, and queue cleanup. Useful for async page extraction trace, not first-turn web search hardcoding. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/extract/helpers/source-tracker.ts` | parsed | extraction provenance through merge/dedupe | Extracted per-item source tracking through transformation, pre-dedupe mapping, and final merged item source mapping. Pattern is useful for claim/evidence provenance, not for forcing output shape. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/search/scrape.test.ts` | parsed | search-plus-scrape test contract | Extracted the requirement that search-attached scrape jobs preserve metadata and may partially fail while still returning useful markdown for at least some results. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/__tests__/snips/v2/search.test.ts` | parsed | structured search result lanes | Extracted `web`, `news`, and `images` as separately bounded result lanes, include-domain behavior, and partial scrape tolerance. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/__tests__/lib/search-query-builder.test.ts` | parsed | request-owned source constraints | Confirmed include/exclude domain filters and category maps are request/policy concerns. We used this for scope bridging, not for copying default research domain lists. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/services/worker/scrape-worker.ts` | parsed | scrape worker lifecycle | Extracted timeout/abort, redirect filtering, dedupe/lock, discovered-link enqueue, billing metadata, and document metadata preservation. Useful for long-running crawl jobs, not first-turn search hardcoding. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/deep-research/research-manager.ts` | parsed | iterative research loop | Extracted query generation from findings, gap analysis, max failed attempts, and final synthesis from accumulated findings. Pattern maps to a future multi-turn research workflow CD, not a fixed prompt route. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/api/src/lib/deep-research/deep-research-service.ts` | parsed | parallel searches plus seen-URL filtering | Extracted bounded depth, parallel query execution, unique URL tracking, source list retention, gap-driven continuation, and final synthesis as the end state. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/js-sdk/firecrawl/src/v2/types.ts` | parsed | typed multi-lane result contract | Confirmed web/news/images rows have separate fields and can also be returned as scraped documents. Used to keep provider rows structured without forcing a final answer shape. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/firecrawl/apps/js-sdk/firecrawl/src/__tests__/e2e/v2/search.test.ts` | parsed | multi-lane search behavior | Confirmed web/news/images may be requested together, bounded by limit, and can contain either search rows or scraped documents. |

## Repo Surfaces Pending

| Area | Paths | Priority | Reason |
| --- | --- | --- | --- |
| Scrape pipeline internals | `apps/api/src/scraper/scrapeURL/**`, `apps/api/src/services/worker/scrape-worker*` | high | Needed for clean markdown/page extraction patterns. |
| Crawl and batch status contracts | SDK methods and deeper worker/redis queue internals | medium | Controller contracts parsed; worker lifecycle still pending before implementation. |
| Map/discovery | deeper crawler/sitemap utilities used by map | medium | Controller and map-utils parsed; sitemap/crawler internals pending if site-discovery becomes the next primitive. |
| Extract/structured data | `apps/api/src/lib/extract/**` | medium | Controller, URL processor, reranker, document scraper, and source tracker parsed; schema/completion internals still pending. |
| Tests and evals | `apps/api/src/__tests__/snips/v2/**`, e2e tests, scrape evals | high | Needed to validate behavior patterns and edge cases. |
| MCP/CLI/skill surfaces | Firecrawl MCP/CLI references and SDK examples | low | Useful after core retrieval primitive improves. |

## Implemented Slices

### `FIRECRAWL-LIVE-001`: high-volume candidate pool policy

Pattern source: `FIRECRAWL-PATTERN-001`, primarily from `apps/api/src/search/execute.ts` and `apps/api/src/search/v2/searxng.ts`.

Target behavior:

- Increase web search candidate volume before filtering.
- Keep final synthesis evidence bounded.
- Keep concurrency bounded so provider fallback chains have enough wall-clock room to return.
- Keep behavior CD/policy-controlled.
- Avoid provider-specific prompt routes or domain-specific answer formats.
- Preserve cache lifecycle controls so tests can opt out of stale cache influence.

Current status: in progress.

Validation:

- First smoke with higher parallelism timed out all subqueries. Adjusted policy toward higher result volume with bounded concurrency and longer per-query budget.
- Second smoke returned provider artifacts instead of timing out: `provider_result_count=7`, `provider_result_dedup_count=17`, `parallel_window=3`.
- Second smoke still produced `status=no_results`; provider coverage remained the bottleneck (`gdelt_doc_rate_limited`, missing Serper key, Bing off-topic, direct fetch denied on some fallbacks).
- Net result: the Firecrawl high-recall pattern improved observability and candidate attempts, but did not yet solve evidence quality. Next useful slice is search-plus-extract/page-content expansion or an optional Firecrawl-compatible provider adapter.

### `FIRECRAWL-LIVE-002`: policy-owned page extraction budget

Pattern source: `FIRECRAWL-PATTERN-002` and `FIRECRAWL-PATTERN-006`, primarily from `apps/api/src/search/scrape.ts`, `apps/api/src/scraper/scrapeURL/index.ts`, and `apps/api/src/scraper/scrapeURL/engines/index.ts`.

Target behavior:

- Treat result-link fetching as a page-extraction step after candidate discovery, not as an implicit one-off fallback.
- Keep the extraction budget declared in the batch-query policy CD.
- Bound extraction by per-stage and per-query budgets.
- Preserve the existing permission/tool boundary by using the current web conduit fetch path.
- Avoid domain-specific routes, fixed output formats, or Firecrawl-only execution.

Current status: implemented first slice.

Implementation:

- Added `batch_query.page_extraction` policy fields: `enabled`, `max_links_per_stage`, `max_total_fetches`, and `trigger`.
- Added `batch_query.page_extraction.extract_mode` so follow-up page extraction can request the current web conduit text or markdown extractor by policy.
- Changed batch-query link follow-up extraction to read those policy fields instead of using a fixed Rust constant.
- Set the runtime policy to allow up to two follow-up page fetches per stage and four per query, and to prefer markdown for fetched page evidence, while keeping the Rust default conservative for missing policy files.
- Added a unit test proving follow-up fetch volume is policy-owned.

Validation:

- `cargo test -p infring-ops-core page_extraction_budget_is_policy_owned -- --nocapture` passed after fixing the test fixture to force the low-signal extraction path.
- Live smoke with `scientific breakthroughs 2026` proved the policy wiring works, but the first five-link extraction budget hit web-conduit `rate_limit_exceeded` on weak Bing links. Policy was reduced to two links per stage/four per query to preserve extraction without saturating the current rate budget.

Open finding:

- The remaining blocker in this smoke was upstream candidate quality: the provider chain returned timeouts, off-topic dictionary/wiki links, GDELT circuit-open responses, and low-signal DuckDuckGo API JSON. Page extraction cannot recover much when the discovered candidate URLs are irrelevant.

### `FIRECRAWL-LIVE-003`: rank-before-extraction threshold

Pattern source: `FIRECRAWL-PATTERN-001`, `FIRECRAWL-PATTERN-002`, and `FIRECRAWL-PATTERN-006`, primarily from `apps/api/src/lib/extract/fire-0/url-processor-f0.ts` and `apps/api/src/lib/extract/fire-0/reranker-f0.ts`.

Target behavior:

- Treat follow-up page fetches as scarce extraction work.
- Rank discovered links before fetching page content.
- Keep the minimum link score in the batch-query policy CD.
- Avoid domain-specific routes, fixed user-facing output formats, or Firecrawl-only mechanics.

Current status: implemented first slice.

Implementation:

- Added `batch_query.page_extraction.min_link_score` to the policy CD and default policy.
- Added a policy-read helper and a page-extraction link selector that applies the configured score threshold before follow-up fetches.
- Kept the existing fallback link preview path unchanged for diagnostics/provider artifacts.
- Added a unit test proving weak one-term links are filtered before extraction while relevant multi-term links remain fetchable.

Expected effect:

- This should reduce rate-budget waste on weak SERP links such as calculators, dictionary pages, generic portals, or login/signup surfaces when only one loose query term overlaps.
- It does not solve provider coverage by itself; it prevents the new extraction budget from amplifying bad candidate discovery.

### `FIRECRAWL-LIVE-004`: request-owned source constraints

Pattern source: `FIRECRAWL-PATTERN-004`, primarily from `apps/api/src/controllers/v2/search.ts` and `apps/api/src/lib/search-query-builder.ts`.

Target behavior:

- Preserve source-scope fields supplied by the user, agent, or workflow CD.
- Bridge those fields through batch query to the existing web conduit search surface.
- Keep domain scoping request-owned rather than introducing a hardcoded research-domain list.
- Keep scoped and unscoped cache identities separate so test runs and live requests do not cross-contaminate.
- Expose the scope in the query contract for diagnostics without forcing any final answer format.

Current status: implemented first slice.

Implementation:

- Added a `BatchQuerySearchScope` request contract for `allowed_domains`/`include_domains` and camelCase aliases.
- Added conservative domain normalization and dedupe for explicit request values only.
- Forwarded `allowed_domains` and `exclude_subdomains` into web conduit search requests.
- Added scoped cache identity for non-empty source constraints, while preserving legacy cache fallback for unscoped requests.
- Added tests for scope normalization, request forwarding, and scoped cache separation.

Expected effect:

- Research workflows can now ask for broader retrieval while still constraining source families when the request already carries that intent.
- This does not add domain-specific research behavior. It gives the agent and workflow CDs a cleaner primitive for source control when they choose to use it.

### `FIRECRAWL-LIVE-005`: structured search-result candidate pools

Pattern source: `FIRECRAWL-PATTERN-001`, `FIRECRAWL-PATTERN-003`, and `FIRECRAWL-PATTERN-006`, primarily from `apps/api/src/__tests__/snips/v2/search.test.ts`, `apps/api/src/search/scrape.ts`, and the SDK search result contracts.

Target behavior:

- Accept AI-friendly provider payloads that already contain structured result arrays.
- Preserve result lanes such as web/news/documents as candidate metadata.
- Prefer many structured candidates plus downstream filtering over one lossy summary string.
- Keep the row budget policy-owned.
- Avoid any domain-specific answer behavior or final response format rules.

Current status: implemented first slice.

Implementation:

- Added `batch_query.structured_results` policy fields for enablement and max rows per stage.
- Added generic structured-result extraction from arrays such as `web`, `news`, `results`, `items`, `organic`, `documents`, `data`, and `links`.
- Candidate extraction reads common row fields (`url`/`link`/`href`, title/name/headline, description/snippet/markdown/content/text) and metadata fallbacks without assuming a specific provider.
- Structured candidates now feed the same eligibility, scoring, dedupe, evidence, and synthesis path as rendered search rows.
- Added tests proving structured Firecrawl-style payloads produce candidates and can synthesize even when the wrapper summary is low-signal.

Expected effect:

- A future Firecrawl-like provider adapter can return `web` rows with markdown/metadata directly, and batch query will consume them as evidence instead of discarding them behind a low-signal wrapper summary.
- This improves the high-volume-filtering path without changing user-facing output format or adding prompt/domain hardcoding.

### `FIRECRAWL-LIVE-006`: retain structured provider rows

Pattern source: `FIRECRAWL-PATTERN-001`, `FIRECRAWL-PATTERN-003`, and `FIRECRAWL-PATTERN-006`, primarily from Firecrawl search tests and SDK result type contracts.

Target behavior:

- Preserve structured provider rows alongside the existing rendered search text.
- Keep web/news/images lanes explicit where the provider payload supports them.
- Let batch query consume structured rows through the same candidate filtering path already used for Firecrawl-style payloads.
- Avoid changing user-facing synthesis format or adding domain-specific research behavior.

Current status: implemented first slice.

Implementation:

- Serper parser now retains filtered organic rows as `web` result objects.
- Bing RSS parser now retains filtered RSS items as `web` result objects.
- GDELT parser now retains filtered article rows as `news` result objects with date/source metadata.
- Batch-query structured row extraction now recognizes image lanes and common image URL field aliases.
- Added tests for provider structured-row retention and multi-lane candidate extraction.

Expected effect:

- Built-in providers can feed the high-volume structured candidate path instead of only relying on rendered summary text.
- This should make retrieval artifacts easier for the agent to inspect and easier for synthesis to ground, while preserving all existing rendered text behavior.
