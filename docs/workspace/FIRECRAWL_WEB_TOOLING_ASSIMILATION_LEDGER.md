# Firecrawl Web Tooling Assimilation Ledger

## Scope

- Source repo: `https://github.com/mendableai/firecrawl` / `https://github.com/firecrawl/firecrawl`
- Local checkout: `local/workspace/shadow/external-repos/firecrawl`
- Revision inspected: `3afe6df`
- Target: extract general web retrieval patterns that can improve Infring web tooling without copying Firecrawl syntax, domain-specific behavior, hosted-service assumptions, or UI/API surface noise.

## Tracking Contract

- `docs/workspace/FIRECRAWL_WEB_TOOLING_FILE_INVENTORY.tsv` is the exhaustive per-file ledger.
- Files are only marked `parsed` after direct file reads.
- Files seen through `rg`, `find`, or directory listings remain `not_parsed` unless opened directly.
- Generated lockfiles and binary/media samples are explicitly marked skipped so they do not hide in the remaining count.
- Assimilation focuses on patterns and control flow, not copying code.
- Primary focus stays on current Infring pain points, but the pass should capture any broadly useful system pattern encountered along the way.

## Assimilation Posture

- Current pain-point focus:
  - web retrieval quality
  - candidate breadth and ranking
  - search-result-to-page enrichment
  - evidence pack quality
  - synthesis handoff usefulness
  - low-signal recovery without prompt/domain hardcoding
- Opportunistic capture:
  - useful architecture boundaries
  - queue/job orchestration patterns
  - retry, timeout, and waterfall mechanics
  - observability, telemetry, and eval patterns
  - security, privacy, ZDR, robots, and blocklist handling
  - document parsing and content extraction
  - test harnesses and failure-mode fixtures
  - cost/budget/accounting controls
  - cache lifecycle and cleanup policies
- Capture rule: opportunistic items should be recorded as patterns with a likely Infring target area, but should not be implemented immediately unless they directly support the current web tooling pain point or are small, low-risk primitives.

## Current Inventory

- Total tracked files: 1357
- Parsed: 161
- Not parsed: 1117
- Skipped generated: 11
- Skipped media or sample: 68

## Parsed Files

| File | Why Parsed | Useful Pattern Signal |
| --- | --- | --- |
| `README.md` | Product-level feature surface. | Search can optionally return full page content, not just search snippets; outputs should be LLM-ready markdown/structured data. |
| `SELF_HOST.md` | Deployment/provider constraints. | Provider stack is environment-dependent; self-hosted modes may lack advanced anti-bot/fire-engine capabilities and need explicit fallback expectations. |
| `CLAUDE.md` | Maintainer workflow and tests. | Firecrawl prefers E2E/snips tests with happy and failure paths around actual API behavior. |
| `apps/api/src/controllers/v2/search.ts` | Public search request entrypoint. | Normalize/validate request, preserve agent interop and ZDR boundaries, then delegate to a single search executor. |
| `apps/api/src/controllers/v2/f-search.ts` | Search-index endpoint. | Index-backed search exposes hybrid, keyword, semantic, and BM25 modes plus domain/country/freshness/language filters as candidate-discovery controls. |
| `apps/api/src/search/execute.ts` | Search orchestration. | Overfetch search results, categorize domains, optionally scrape selected candidates, then merge scraped content back into the search response. |
| `apps/api/src/search/scrape.ts` | Search-result enrichment. | Treat SERP rows as candidates; filter blocked URLs, directly scrape result URLs concurrently, and merge richer documents into original result rows. |
| `apps/api/src/search/v2/index.ts` | Provider fallback. | Try configured premium engine first, then self-hosted SearXNG, then DuckDuckGo, failing to an empty response rather than leaking provider errors. |
| `apps/api/src/scraper/scrapeURL/index.ts` | Scrape pipeline entrypoint. | Build feature flags from requested output, choose engine waterfall, use prefetch/robots/transformers/postprocessors as a page-to-document pipeline. |
| `apps/api/src/search/v2/ddgsearch.ts` | DuckDuckGo provider implementation. | HTML endpoint pagination, seen-URL dedupe, anti-bot retry, timeout handling, and user-agent rotation are provider-internal mechanics. |
| `apps/api/src/search/v2/searxng.ts` | SearXNG provider implementation. | Fetch enough pages to satisfy requested result count, stop on empty pages, normalize provider rows into a shared web result shape. |
| `apps/api/src/search/v2/fireEngine-v2.ts` | Premium search provider implementation. | Provider calls are retried behind an adapter boundary and can return partial source-type coverage. |
| `apps/api/src/lib/search-query-builder.ts` | Structured query filter builder. | Categories, include domains, exclude domains, and PDF filters compile into query/filter lanes while preserving a category map for result metadata. |
| `apps/api/src/lib/search-index-client.ts` | Managed search-index client. | Optional index search/index writes fail closed to empty candidates or logged diagnostics, carry score/freshness/quality/rank metadata, and must not block scraping. |
| `apps/api/src/controllers/v2/types.ts` | Request schema and search options. | Strict request schema supports sources, categories, domains, scrape options, timeout bounds, and format defaults as typed input rather than prompt phrasing. |
| `apps/api/src/controllers/v2/browser.ts` | Browser session controller. | Dynamic browser work is TTL/concurrency bounded, owner checked, retried on creation, cleaned up if persistence fails, and finalized with idempotent destroyed-state claiming. |
| `apps/api/src/controllers/v2/scrape-browser.ts` | Scrape interaction controller. | Interactive work can replay a prior scrape context, adopt or create a bounded session, then run prompt/code interaction with trace hygiene and explicit cleanup. |
| `apps/api/src/controllers/v2/agent.ts` | Agent passthrough controller. | Agent jobs are admitted as async handles with request logging and status polling, while unsupported zero-retention and unavailable beta services fail closed. |
| `apps/api/src/controllers/v2/agent-status.ts` | Agent status projection. | Agent status returns processing/completed/failed plus expiry and result data only after ownership checks and terminal job lookup. |
| `apps/api/src/controllers/v2/agent-cancel.ts` | Agent cancellation controller. | Cancellation checks ownership and refuses already-finished work before forwarding a delete to the backing service. |
| `apps/api/src/__tests__/snips/v2/search.test.ts` | Search E2E behavior tests. | Tests cover include/exclude domain behavior, limits, multi-source search, and search-plus-scrape enrichment with partial scrape tolerance. |
| `apps/api/src/scraper/scrapeURL/engines/index.ts` | Scrape engine waterfall. | Engines declare feature support and quality; selection ranks by requested capabilities and quality, with special-case engines hidden behind capability policy. |
| `apps/api/src/scraper/scrapeURL/engines/fetch/index.ts` | Plain fetch engine. | Fetch path detects charset from headers/meta, uses secure dispatcher, preserves status/content-type, and runs specialty-content checks. |
| `apps/api/src/scraper/scrapeURL/engines/index/index.ts` | Cache/index engine. | Cache/index reads are gated by request features, ZDR, headers, actions, freshness, normalized URL hashes, and async write-back. |
| `apps/api/src/scraper/scrapeURL/engines/document/index.ts` | Document engine. | Non-HTML documents are detected by content-type or URL extension, converted to HTML, and temporary prefetched files are cleaned up. |
| `apps/api/src/lib/ranker.ts` | Semantic candidate ranking. | Embedding/cosine rerank can improve candidate order while falling back safely and preserving stable order for ties. |
| `apps/api/src/lib/map-utils.ts` | Map/search URL discovery. | Index, sitemap, and search-map candidates are merged; map rows can be semantically ranked, domain/path filtered, and deduped while preserving better title metadata. |
| `apps/api/src/lib/canonical-url.ts` | URL equivalence utility. | Canonicalization strips protocol, `www`, and trailing slash for low-cost duplicate/equivalence checks. |
| `apps/api/src/lib/robots-txt.ts` | Robots and sitemap policy. | Robots retrieval uses an engine waterfall, treats 404 as empty policy, and exposes crawl-delay/sitemap metadata for crawler planning. |
| `apps/api/src/scraper/WebScraper/crawler.ts` | Crawl candidate selection and budget hygiene. | Candidate links are filtered for domain, depth, robots, include/exclude rules, protocol, assets, and sitemap dedupe before spending crawl/fetch budget. |
| `apps/api/src/scraper/scrapeURL/lib/extractLinks.ts` | Link extraction. | Use a fast parser when available, fall back to tolerant HTML parsing, resolve `<base href>`, skip fragment-only anchors, and dedupe resolved absolute links. |
| `apps/api/src/scraper/scrapeURL/lib/extractMetadata.ts` | Page metadata extraction. | Extract title, description, favicon, language, keywords, robots, OpenGraph/DC/article metadata, and custom meta as lightweight evidence quality signals. |
| `apps/api/src/scraper/WebScraper/utils/blocklist.ts` | URL blocklist policy. | Domain/subdomain and related-TLD blocking can be overridden by allow keywords/regex, keeping block decisions policy-driven. |
| `apps/api/src/scraper/scrapeURL/engines/utils/safeFetch.ts` | Secure fetch dispatcher. | Fetch transport guards private-network targets at connection time and separates cookie-bearing scraping from no-cookie delivery modes. |
| `apps/api/src/scraper/scrapeURL/engines/utils/specialtyHandler.ts` | Specialty content lane split. | Content type and magic signatures route PDFs/documents to specialty extractors and reject unsupported binary formats instead of treating them as page evidence. |
| `apps/api/src/lib/html-to-markdown.ts` | HTML-to-markdown waterfall. | Conversion can try service/native parser first, then tolerant markdown conversion, with postprocessing and ZDR-aware logging. |
| `apps/api/src/lib/html-to-markdown-client.ts` | HTML-to-markdown service call. | Request IDs and sizes are omitted under ZDR; converter failures fall back rather than becoming final visible failures. |
| `apps/api/src/scraper/scrapeURL/lib/removeUnwantedElements.ts` | HTML cleanup. | Prefer native transform, fall back to include/exclude selectors, strip shell/noise elements, absolutize links/images, and choose largest srcset image. |
| `apps/api/src/scraper/scrapeURL/lib/rewriteUrl.ts` | Processible URL rewrites. | Some document-host URLs can be rewritten to HTML/download forms before extraction; useful as a capability-specific adapter pattern, not a research-domain route. |
| `apps/api/src/scraper/scrapeURL/lib/fetch.ts` | Robust fetch wrapper. | Retries, schema validation, abort propagation, cacheable DNS, mock recording, and sensitive body redaction belong inside adapter mechanics. |
| `apps/api/src/scraper/scrapeURL/retryTracker.ts` | Retry budget tracker. | Per-reason budgets keep recovery bounded across feature toggles, feature removals, and PDF/document prefetch retries. |
| `apps/api/src/scraper/scrapeURL/shouldCheckRobots.ts` | Robots lockdown rule. | Lockdown mode must not perform even a robots fetch; robots checks should be an explicit policy/team flag. |
| `apps/api/src/scraper/scrapeURL/engines/playwright/index.ts` | Rendered page engine. | Rendered extraction is a capability lane with wait budget, timeout, headers, TLS setting, and JSON-inner-content cleanup. |
| `apps/api/src/scraper/scrapeURL/engines/pdf/index.ts` | PDF extraction ladder. | Validate PDF magic bytes, try fast/native extraction only when confidence is high, then fall back to OCR/MinerU/pdf-parse lanes with page/time budgets. |
| `apps/api/src/scraper/scrapeURL/engines/pdf/pdfParse.ts` | PDF text fallback. | Simple text fallback escapes output and records duration/page count; useful as a low-complexity document lane. |
| `apps/api/src/scraper/scrapeURL/engines/pdf/pdfUtils.ts` | PDF magic sniffing. | Check for `%PDF` inside the first 1KB rather than trusting URL extension or content type alone. |
| `apps/api/src/scraper/WebScraper/sitemap.ts` | Sitemap discovery pipeline. | Recursively process sitemap indexes with a hit cap, gzip support, parser fallback, URL/file filtering, and batched URL handoff. |
| `apps/api/src/scraper/WebScraper/utils/maxDepthUtils.ts` | Crawl depth utility. | Depth budgets should be relative to the seed URL and ignore index filenames. |
| `apps/api/src/lib/validateUrl.ts` | URL validation and redirect utilities. | Add missing protocol, restrict to HTTP(S), reject malformed repeated protocols, and resolve redirects with bounded HEAD/GET attempts. |
| `apps/api/src/lib/url-utils.ts` | Public-suffix URL utilities. | Base-domain detection should handle multi-part TLDs and distinguish root domains from subdomains/paths. |
| `apps/api/src/scraper/scrapeURL/lib/extractImages.ts` | Image extraction. | Resolve image candidates through `<base href>`, `srcset`, metadata, icons, background images, and poster attributes while filtering invalid schemes. |
| `apps/api/src/scraper/scrapeURL/lib/extractAttributes.ts` | Attribute extraction. | Use fast/native extraction with a tolerant selector/attribute fallback and bounded debug summaries. |
| `apps/api/src/scraper/scrapeURL/lib/extractSmartScrape.ts` | Interactive extraction decider. | Before invoking expensive interaction, ask whether missing structured data requires page interaction, carry reasoning/prompt internally, and keep cost-bounded fallbacks. |
| `apps/api/src/scraper/scrapeURL/lib/smartScrape.ts` | Interactive scrape adapter. | Validate agent endpoint responses, track cost, map failures, and keep model/tool choices inside adapter policy rather than user-facing output. |
| `apps/api/src/scraper/scrapeURL/lib/urlSpecificParams.ts` | URL-specific adapter overrides. | Domain override tables are useful as explicit migration debt/policy exceptions, not as general research logic. |
| `apps/api/src/scraper/scrapeURL/engines/utils/downloadFile.ts` | Download-file transport. | Streaming downloads should map network/TLS/DNS errors and clean temporary files on failure. |
| `apps/api/src/scraper/WebScraper/utils/engine-forcing.ts` | Engine forcing policy. | Capability overrides can be configured by domain/wildcard at the adapter boundary, but should stay explicit policy. |
| `apps/api/src/lib/retry-utils.ts` | Generic retry helper. | Retry loops should use an abortable backoff, a valid-result predicate, and internal observability. |
| `apps/api/src/scraper/scrapeURL/lib/cacheableLookup.ts` | DNS lookup cache. | DNS caching belongs at the transport boundary and should be bypassable in dev/test. |
| `apps/api/src/scraper/WebScraper/__tests__/dns.test.ts` | DNS cache behavior test. | Test that DNS caching is actually installed in the HTTP transport path. |
| `apps/api/src/lib/__tests__/url-utils.test.ts` | Public-suffix URL tests. | Tests should cover base domains, subdomains, multi-part TLDs, paths, and invalid URLs. |
| `apps/api/src/controllers/v1/__tests__/urlValidation.test.ts` | URL validation behavior tests. | Validation should cover protocol defaults, allowed schemes, malformed inputs, and internationalized domains. |
| `apps/api/src/__tests__/snips/v2/crawl.test.ts` | Crawl E2E behavior tests. | Tests verify sitemap modes, sitemap-only subset behavior, start-URL fallback, include-path filtering, and URL normalization. |
| `apps/api/native/src/html.rs` | Native HTML transformer. | Main-content cleanup, metadata backfill, base-href resolution, largest `srcset` image selection, link/image absolutization, and attribute extraction belong in a page-to-document primitive. |
| `apps/api/native/src/crawler.rs` | Native crawler filtering. | Link filtering should return denial reasons for depth/domain/file/protocol/robots/include/exclude decisions, and sitemap parsing should compile recurse/process instructions. |
| `apps/api/native/src/engpicker.rs` | Engine picker quality probe. | Compare cheap fetch output against rendered output with similarity thresholds and an uncertain verdict rather than hard-forcing browser rendering everywhere. |
| `apps/api/native/src/utils.rs` | Native error adapter. | Boundary adapters should convert native failures into typed outer-runtime errors. |
| `apps/api/native/src/lib.rs` | Native module exports. | Keep native extraction/export surfaces explicit and narrow. |
| `apps/api/native/src/pdf.rs` | Native PDF processor. | Separate fast detect-only PDF classification from full processing, preserve confidence/page metadata, and attach structured logs. |
| `apps/api/native/src/document/mod.rs` | Native document conversion entrypoint. | Route document bytes through a provider factory into a normalized document model, then render into an LLM-processible artifact. |
| `apps/api/native/src/document/model/mod.rs` | Normalized document model. | Preserve paragraphs, headings, lists, tables, images, links, code, notes, comments, and metadata as an intermediate representation before text/HTML output. |
| `apps/api/native/src/document/providers/factory.rs` | Document provider selection. | Select document parsers by declared document type rather than prompt wording or URL/domain guesses. |
| `apps/api/native/src/document/providers/mod.rs` | Document provider trait. | Keep document parsing behind a narrow parse-buffer trait so each format lane can fail independently. |
| `apps/api/native/src/document/providers/doc.rs` | Legacy DOC parser. | For older binary documents, try metadata streams first, then bounded CP1252/UTF-16 text-run extraction with stream fallbacks and minimum word thresholds. |
| `apps/api/native/src/document/providers/docx.rs` | DOCX parser. | Parse zip XML parts into structural blocks, preserving relationships, styles, headings, lists, tables, links, notes, comments, and external images. |
| `apps/api/native/src/document/providers/odt.rs` | ODT parser. | Harvest styles/content from multiple XML parts, detect headings/lists/tables/notes/comments, and ignore invisible empty structure. |
| `apps/api/native/src/document/providers/rtf.rs` | RTF parser. | Decode control words, Unicode escapes, style state, paragraphs, and simple tables while skipping non-content destinations. |
| `apps/api/native/src/document/providers/xlsx.rs` | Spreadsheet parser. | Convert sheets into heading plus table blocks so tabular evidence stays structured instead of flattened into prose. |
| `apps/api/native/src/document/renderers/html.rs` | Document HTML renderer. | Render the normalized document model into safe structural HTML with title/author metadata, tables, notes, comments, and inline formatting. |
| `apps/api/native/src/document/renderers/mod.rs` | Renderer module. | Keep renderer surfaces explicit and swappable after normalized extraction. |
| `apps/api/src/__tests__/snips/v2/document-converter.test.ts` | Document converter tests. | Exact-output tests lock DOCX/ODT/RTF/XLSX conversion into stable structural HTML. |
| `apps/api/src/__tests__/snips/v2/parsers.test.ts` | Parser option tests. | Parser selection supports typed object options such as PDF mode/max pages, rejects invalid shorthands, and bills by actual document work. |
| `apps/api/src/__tests__/snips/v2/parse.test.ts` | Upload parse tests. | Parse-only workflows accept uploaded HTML/DOCX/PDF but reject options that imply browsing, interaction, location, screenshots, or unsupported media. |
| `apps/api/src/__tests__/snips/v2/scrape-formats.test.ts` | Scrape format tests. | String and object format declarations should converge to the same output structure; format options are typed contracts, not final-answer templates. |
| `apps/api/src/__tests__/snips/v2/scrape-cache.test.ts` | Cache behavior tests. | Cache controls need explicit maxAge/minAge semantics and observable hit/miss state so retrieval tests can avoid stale artifacts. |
| `apps/api/src/__tests__/snips/v2/scrape-lockdown.test.ts` | Lockdown mode tests. | Lockdown should serve only admitted cached artifacts, avoid outbound side-effect lanes, and behave as ZDR. |
| `apps/api/src/__tests__/snips/v2/scrape-query.test.ts` | Page query format tests. | Page-local question/highlight extraction is a format lane over retrieved content, distinct from broad web search and bounded by prompt-length validation. |
| `apps/api/src/controllers/v2/parse.ts` | Upload parse controller. | Parse-only lanes classify uploaded HTML/PDF/office files by filename/content type, force the matching extraction engine, disable cache storage, reject browsing/rendering options, and log sanitized file metadata. |
| `apps/api/src/lib/browser-sessions.ts` | Browser session state helpers. | Session rows track TTL, owner, status, CDP/view handles, prompt-use flags, cached active counts, and idempotent destroyed-state claiming. |
| `apps/api/src/lib/browser-session-activity.ts` | Browser activity batching. | Browser execution telemetry is queued and batch-inserted as internal activity records instead of becoming retrieval evidence. |
| `apps/api/src/lib/scrape-interact/browser-service-client.ts` | Browser service client. | Browser service calls are behind a narrow typed adapter that throws typed non-2xx errors and keeps service URLs/headers internal. |
| `apps/api/src/lib/scrape-interact/scrape-replay.ts` | Scrape replay builder. | Prior scrape context is reconstructed only from retained URL/options, actions are sanitized and waits clamped, and replay scripts skip output-only actions. |
| `apps/api/src/lib/scrape-interact/browser-agent.ts` | Browser agent loop. | Interaction starts from current URL plus accessibility snapshot, bounds steps/time, resnapshots after state changes, blocks new tabs/out-of-scope opens, and returns extracted output rather than raw trees. |
| `apps/api/src/lib/scrape-interact/langsmith.ts` | Interaction trace wrapper. | External tracing is opt-in, strips URL query/fragment metadata, and is disabled for zero-retention contexts. |
| `apps/api/src/lib/scrape-interact/langsmith.test.ts` | Trace hygiene tests. | Tests verify disabled-by-default tracing, whitespace-key handling, zero-retention skip behavior, raw SDK fallback, and URL sanitization. |
| `apps/api/src/lib/format-utils.ts` | Format option helper. | Format checks need to support both string and typed object declarations while staying extraction-contract logic, not final-answer formatting. |
| `apps/api/src/lib/extract/build-document.ts` | Extract document builder. | LLM extraction input appends sanitized, bounded page metadata to markdown so source context travels with content without raw metadata bloat. |
| `apps/api/src/lib/extract/document-scraper.ts` | Extraction scrape bridge. | Scrape-for-extract retries single URLs with a larger timeout, tracks URL traces/content stats, bypasses billing internally, and removes transient queue jobs after completion. |
| `apps/api/src/lib/extract/fire-0/build-document-f0.ts` | Alternate extract document builder. | Duplicate extraction lanes keep the same metadata-sanitization boundary, which is useful as a compatibility invariant. |
| `apps/api/src/lib/extract/fire-0/document-scraper-f0.ts` | Alternate extraction scrape bridge. | Extraction scrape behavior remains the same across provider versions: blocklist first, queue job, wait, cleanup, trace content stats. |
| `apps/api/src/scraper/scrapeURL/error.ts` | Scrape error taxonomy. | Typed recoverable errors distinguish unsupported binary, DNS, no cache, lockdown miss, ZDR violation, PDF OCR/time, PDF/document antibot, feature retry limits, and engine waterfall control. |
| `apps/api/src/lib/deep-research/research-manager.ts` | Deep research state and LLM planning. | Maintain seen URLs/findings/sources/depth/failure counts, generate 3-5 specific follow-up queries from prior findings, analyze gaps, and reserve time for final synthesis. |
| `apps/api/src/lib/deep-research/deep-research-service.ts` | Deep research orchestration loop. | Bounded loop: generate parallel query lanes, search-plus-scrape them, dedupe URLs, append findings/sources, analyze coverage gaps, continue or synthesize under max-depth/max-URL/time budgets. |
| `apps/api/src/lib/deep-research/deep-research-redis.ts` | Deep research state persistence. | Long-running research state uses TTL-bounded storage and appends activities/sources/summaries while keeping findings locally capped. |
| `apps/api/src/controllers/v1/search.ts` | V1 search and deep-research search bridge. | Deep research uses a small SERP result set plus scrape enrichment; normal search filters content-bearing docs when scrape formats are requested. |
| `apps/api/src/search/transform.ts` | Search response transformation. | Multi-source search rows are normalized into a common document shape, and content-bearing rows are separated from thin SERP metadata. |
| `apps/api/src/search/index.ts` | V1 search provider fallback. | Provider order falls through Fire Engine, SearXNG, then DuckDuckGo, returning empty on failures rather than exposing provider internals. |
| `apps/api/src/lib/ranker.test.ts` | Ranking behavior tests. | Semantic rerank should expose scores/original indices, handle empty inputs, and preserve stable order for equal scores. |
| `apps/api/src/search/fireEngine.ts` | Legacy Fire Engine search adapter. | Search adapters should retry until a valid non-empty result predicate succeeds and otherwise return an empty set behind the provider boundary. |
| `apps/api/src/search/searxng.ts` | Legacy SearXNG adapter. | Requested result counts can be satisfied by fetching multiple result pages and stopping early on empty pages. |
| `apps/api/src/controllers/v1/deep-research.ts` | Deep research request controller. | Public deep-research inputs are bounded by max depth, max URLs, and time limit, then queued as an async job with initialized progress state. |
| `apps/api/src/controllers/v1/deep-research-status.ts` | Deep research status controller. | Research jobs expose final analysis, sources, activities, expiry, depth, and status as a projection, not raw queue state. |
| `apps/api/src/services/queue-service.ts` | Queue lifecycle configuration. | Long-running research queues should have explicit remove-on-complete/fail ages so async artifacts do not grow without lifecycle bounds. |
| `apps/api/src/lib/generate-llmstxt/generate-llmstxt-service.ts` | Site corpus generation service. | Map a bounded URL set, scrape pages in batches, derive compact page descriptions, keep full text separately, update progress after each batch, and cache larger corpus packs for smaller requests. |
| `apps/api/src/lib/generate-llmstxt/generate-llmstxt-redis.ts` | Site corpus generation state. | Async corpus-pack state is TTL-bounded and stores compact generated text, full text, status, cache flag, and errors. |
| `apps/api/src/lib/generate-llmstxt/generate-llmstxt-supabase.ts` | Site corpus cache. | Cache lookup normalizes to hostname, can reuse a larger cached corpus for a smaller requested limit, and rejects week-old corpus entries. |
| `apps/api/src/controllers/v1/generate-llmstxt.ts` | Site corpus request controller. | Corpus generation is async, ZDR-gated, initialized with max URL/show-full/cache options, and projected by job ID. |
| `apps/api/src/controllers/v1/generate-llmstxt-status.ts` | Site corpus status controller. | Status projection can return only compact index or compact plus full text based on request, with expiry and failure state. |
| `apps/api/src/controllers/v2/crawl.ts` | V2 crawl request controller. | Explicit crawler options override agent/planner-generated options, invalid path regexes fail early, robots are fetched before queueing, and crawl groups get TTL-bounded handles. |
| `apps/api/src/controllers/v2/map.ts` | V2 map request controller. | Map is treated as lightweight URL discovery with timeout/cancel support, optional index/search/sitemap sources, and low-result warnings that can drive broader retrieval. |
| `apps/api/src/controllers/v2/crawl-params-preview.ts` | Crawl option preview controller. | Site structure can be discovered before option planning, but option generation should remain optional and never override explicit request fields. |
| `apps/api/src/controllers/v2/crawl-status.ts` | V2 crawl status projection. | Status returns counts, expiry, bounded result windows, next cursors, and warnings rather than raw queue state or unbounded crawl payloads. |
| `apps/api/src/controllers/v2/crawl-cancel.ts` | Crawl cancellation controller. | Cancellation is persisted as crawl state after ownership checks and should project as a terminal workflow state. |
| `apps/api/src/controllers/v2/crawl-errors.ts` | Crawl error projection. | Failed page errors are projected as sanitized per-URL summaries, while robots-blocked URLs are separated as access-policy evidence. |
| `apps/api/src/controllers/v2/crawl-ongoing.ts` | Ongoing crawl projection. | Ongoing async work is listed by handle, origin URL, creation time, and options without exposing queue internals. |
| `apps/api/src/controllers/v2/crawl-status-ws.ts` | Streaming crawl status projection. | Streaming results begin with a catchup projection, then emit bounded document events and done/error terminal messages. |
| `apps/api/src/lib/crawl-redis.ts` | Crawl state and URL locking. | Crawl state is TTL-bounded; URL locks normalize/canonicalize variants, enforce limits before work, track visited sets, and clear visited memory at finish. |
| `apps/api/src/services/worker/crawl-logic.ts` | Crawl finish aggregation. | Completion aggregates finished page refs/counts and emits completion events without embedding full data in newer webhook-style projections. |
| `apps/api/src/controllers/v1/map.ts` | V1 map implementation. | Discovery merges index, sitemap, and search-map results, uses path/domain filters, dedupes URL variants, and optionally ranks links against a search term. |
| `apps/api/src/controllers/v1/crawl.ts` | V1 crawl controller. | The older crawl path confirms the same primitive: validate options, fetch robots, create a TTL group, save crawl state, enqueue kickoff, and return a handle. |
| `apps/api/src/controllers/v1/crawl-status.ts` | V1 crawl status projection. | Status pagination enforces a byte ceiling and cursor-style next URL so large crawls do not become unbounded chat-visible payloads. |
| `apps/api/src/__tests__/snips/v2/map.test.ts` | Map E2E behavior tests. | Tests cover timeout handling, query-parameter preservation, sitemap-only limits, base-domain warnings, and redirect-normalized mapping. |
| `apps/api/src/__tests__/snips/v2/crawl-prompt.test.ts` | Crawl prompt test placeholder. | The intended behavior is explicit-option precedence, graceful invalid-prompt handling, and schema acceptance, but this file is currently a weak placeholder. |
| `apps/api/src/controllers/__tests__/crawl.test.ts` | Legacy crawl controller test. | Idempotency keys prevent duplicate crawl kickoff requests, a useful retry-safety primitive for async retrieval. |
| `apps/api/src/scraper/WebScraper/__tests__/crawler.test.ts` | WebCrawler unit tests. | Tests lock limit enforcement plus include/exclude behavior across subdomains and full-URL regex modes. |
| `apps/api/src/controllers/v2/batch-scrape.ts` | V2 batch scrape controller. | Batch URL reads validate/ignore invalid URLs explicitly, lock normalized URLs before queueing, support append-to-existing handles, and return a handle plus invalid URL projection. |
| `apps/api/src/controllers/v1/batch-scrape.ts` | V1 batch scrape controller. | The older path confirms the same batch primitive: prevalidate URLs, TTL-bound a group, lock URLs, enqueue single-url jobs, and report a status handle. |
| `apps/api/src/controllers/v2/extract.ts` | V2 structured extract controller. | Structured extraction is its own async job lane with URL block filtering, ZDR rejection, status initialization, and optional invalid URL reporting. |
| `apps/api/src/controllers/v2/extract-status.ts` | V2 extract status projection. | Status loads result data only when complete and projects optional steps/sources/cost/session fields by explicit show flags. |
| `apps/api/src/controllers/v1/extract.ts` | V1 structured extract controller. | Compatibility extraction can run old direct extraction or queued extraction, but both preserve started/completed/failed events and sanitized request state. |
| `apps/api/src/controllers/v1/extract-status.ts` | V1 extract status projection. | Extract status enforces ownership, falls back from Redis to durable store, and returns status/result/error/expiry without raw worker state. |
| `apps/api/src/__tests__/snips/v2/batch-scrape.test.ts` | Batch scrape E2E behavior tests. | Batch reads should return content-bearing documents, preserve original source URLs, and support typed JSON extraction formats. |
| `apps/api/src/lib/extract/extract-redis.ts` | Extract state persistence. | Extract progress is TTL-bounded, stores only recent steps, caps discovered links per step, and separates result storage from status storage. |
| `apps/api/src/lib/extract/extraction-service.ts` | Structured extraction orchestration. | Extraction maps candidate URLs, broadens when mapping is too sparse, chunks multi-entity work, tracks source refs, dedupes/merges results, and returns URL trace/sources when requested. |
| `apps/api/src/lib/extract/url-processor.ts` | Extract URL discovery and rerank. | Site-scope extraction maps URLs, retries a broader map when unique candidates are too few, caps initial candidates, reranks large pools, and records trace status/used-in-completion flags. |
| `apps/api/src/services/queue-jobs.ts` | Scrape job admission and wait path. | Jobs are split by team/run concurrency, overflow moves to bounded backlog, trace context is propagated, queue-full states are explicit, and waiters fall back to durable result storage when needed. |
| `apps/api/src/lib/concurrency-limit.ts` | Concurrency queue state machine. | Active and queued work use TTL-scored sets; crawl-specific limits are checked before promotion; expired/orphaned entries are cleaned; completed jobs release state and promote one eligible successor. |
| `apps/api/src/lib/concurrency-queue-reconciler.ts` | Backlog/runtime drift repair. | A reconciler scans durable backlog rows and Redis queue indexes, requeues missing jobs, promotes eligible work, separates crawl/extract capacity, and bounds stale-entry skips. |
| `apps/api/src/services/worker/nuq.ts` | Durable queue primitive. | Queue rows support backlog promotion, idempotent add-if-missing, lock-based active claims, lock renewal, terminal completion/failure, listen/poll wait modes, metrics, and owner-scoped groups. |
| `apps/api/src/services/worker/nuq-worker.ts` | Queue worker loop. | Workers expose health/metrics, fetch active work with backoff, renew locks during processing, and mark completion/failure only through lock-checked queue APIs. |
| `apps/api/src/services/worker/nuq-prefetch-worker.ts` | Queue prefetch worker. | Prefetching decouples durable queued rows from worker pickup while keeping health/metrics and graceful shutdown. |
| `apps/api/src/services/worker/nuq-reconciler-worker.ts` | Queue reconciliation worker. | Reconciliation runs on a bounded interval, avoids overlapping runs, exposes recovery metrics, and waits for in-flight reconciliation before shutdown. |
| `apps/api/src/services/worker/scrape-worker.ts` | Retrieval worker and incremental crawl expansion. | Completed pages can discover more links, but new work is enqueued only after crawl policy filtering, robots recording, URL locks, priority assignment, and parent-run cancellation checks. |
| `apps/api/src/lib/job-priority.ts` | Queue priority helper. | Priority adapts to recent per-team queue pressure with a TTL window rather than static one-size scheduling. |
| `apps/api/src/services/idempotency/create.ts` | Idempotency key creation. | Idempotency keys are persisted at request admission so retries can be detected before duplicate work starts. |
| `apps/api/src/services/idempotency/validate.ts` | Idempotency key validation. | Request retry safety validates UUID-shaped keys and treats existing keys as duplicate work admission. |
| `apps/api/src/services/extract-queue.ts` | Extract queue and DLQ. | Extraction work uses persistent message IDs, prefetch bounds, explicit ack/nack, single-delivery DLQ routing, and DLQ requeue only when DLQ handling itself fails. |
| `apps/api/src/services/extract-worker.ts` | Extract worker terminal status. | Extraction emits started/completed/failed state, persists sanitized terminal failures, acks handled errors, and marks crashed DLQ work failed instead of retrying forever. |
| `apps/api/src/services/queue-worker.ts` | General worker loop. | Workers gate job pickup on liveness/resource pressure, extend locks for long work, track running jobs, and wait for in-flight jobs during graceful shutdown. |
| `apps/api/src/services/indexing/indexer-queue.ts` | Indexer queue publisher. | Optional index publication no-ops when disabled, reconnects after transport close, sends persistent messages, and waits for bounded drain on backpressure. |
| `apps/api/src/services/indexing/index-worker.ts` | Index/backfill worker. | Budgeted precrawl ranks domains and pages by observed demand, batches URL lookup with backoff, allocates crawl budget proportionally, and submits cacheable crawl jobs only within resource bounds. |

## Decisions So Far

- High-value pattern: `search candidates -> select/filter URLs -> scrape/fetch candidate pages -> merge richer page content -> synthesize from document-like evidence`.
- This is a general primitive for any research domain. It should not be tied to software/framework prompts.
- Search snippets alone should be treated as discovery metadata unless they are already substantial enough for synthesis.
- Provider fallback should remain tooling policy, not visible final-answer phrasing.
- Structured request lanes are valuable: domains, source types, categories, formats, and time/freshness hints should be explicit tool inputs or CD policy fields, not inferred from brittle user prompt phrases.
- Engine/adaptor selection should be capability-driven: each retrieval adapter declares what it can satisfy, and policy ranks viable candidates by quality and budget.
- Enrichment tests should tolerate partial page-fetch misses as long as at least some search candidates become richer document evidence.
- Candidate discovery and enrichment should spend fetch budget only after URL hygiene: normalize document URLs, drop fragments for dedupe, reject non-http schemes, skip search-engine indirections, and avoid asset/archive file types.
- Page-enriched rows should replace thin search rows when they share the same locator and carry denser text, but the marker must stay internal evidence metadata rather than user-visible provider phrasing.
- Robots, sitemap, link extraction, and metadata extraction are useful general retrieval primitives, not research-domain behavior; keep them as tool/CD policy inputs and hidden evidence-pack signals.
- Unsupported binary content should be a lane-split signal, not a successful empty web document. Web fetch can fail closed with `unsupported_content_type` while other media/document/PDF tools handle richer extraction.
- Retry budgets should be per-reason, not only global; this avoids endless recovery loops while allowing one bounded attempt for the specific missing capability.
- Rendered-page and PDF/document extraction should remain separate capability lanes that can be invoked by policy/tool CD when evidence value justifies the cost.
- Fetch-link dedupe should canonicalize URL variants before budget spend: strip fragments, collapse `www`, prefer HTTPS over HTTP, and prefer non-`www` when the document key is otherwise identical.
- Domain-specific engine forcing and URL-specific params are useful only as explicit policy exceptions; they should not become hidden hardcoded research behavior.
- Link filtering should produce internal denial reasons, not just disappear candidates; those reasons can improve retrieval telemetry and help the agent adapt its query/fetch plan without exposing raw tool trace.
- Browser/rendered fetching should be selected by evidence quality probes and capability policy, not by default. A cheap-fetch-vs-rendered similarity probe is a general way to decide when rendering is worth the cost.
- Document-like content should enter document lanes instead of becoming empty/unsupported web evidence. The useful primitive is a normalized document artifact with structural blocks, metadata, page/sheet counts, and extraction quality signals.
- Format/parser choices are typed retrieval contracts. They can influence extraction, billing/budgeting, and evidence shape, but they should not prescribe final answer format or phraseology.
- Broad research benefits from a stateful but bounded gap loop: retain seen locators, evidence refs, source refs, current gaps, depth, elapsed time, and failed attempts; generate follow-up query lanes from collected evidence; stop by sufficiency or explicit gap reasons.
- Firecrawl's deep-research final synthesis contains report-format and model-selection choices that are not good assimilation targets for Infring. The valid pattern is the retrieval/planning loop, not the fixed markdown report style or hardcoded final model.
- Site-level research can benefit from a hidden corpus-pack primitive: map candidate URLs, scrape/read selected pages in batches, keep compact page summaries for navigation and full text behind evidence refs, then let synthesis pull from refs. The valid primitive is the corpus pack, not a forced `llms.txt` visible output.
- Async crawl/map is best treated as three primitives, not one opaque tool: URL discovery, selected page/document extraction, and bounded status/result projection. A handle, queue state, or completed count is not evidence until completed pages are converted into evidence refs.
- Agent/planner-generated crawl options are safe only as optional proposals. Explicit user/workflow fields must win, and failures in option planning should not be required for ordinary research retrieval.
- High-volume retrieval should be candidate-first, not answer-first: broaden discovery when coverage is too sparse, cap and rerank large candidate pools, then spend read/scrape budget only on selected candidates with traceable rejection/selection reasons.
- Structured extraction patterns are useful for evidence tooling, but Firecrawl's prompt-to-schema/model behavior is not an assimilation target for user-facing research; the useful primitive is source-tracked extraction over already-retrieved documents.
- High-volume retrieval needs an execution lifecycle primitive: admission, backlog, active locks, lock renewal, completion/failure, cleanup, and reconciliation must be internal and bounded before the evidence pack trusts completed page refs.
- Incremental crawl expansion is useful only after policy filters and dedupe locks. The primitive is "completed evidence can discover more candidate evidence," not uncontrolled recursive crawling.
- Managed or local search indexes are useful candidate-discovery lanes, especially with hybrid/keyword/semantic/BM25 modes and freshness/quality/rank metadata. They are not source-of-truth lanes; live page/document evidence still backs claims.
- Background corpus warming can improve real-user retrieval when public/storage-permitted sources are safe to cache, but warming queues, demand scores, and index hit counts stay internal and never imply evidence sufficiency.
- Crashed or dead-lettered extraction work should become explicit terminal failure artifacts with sanitized error summaries and missing-evidence reasons, so synthesis gets a gap reason instead of silence or endless retries.
- Dynamic browser interaction is a capability lane, not the default retrieval path. It should activate from evidence-state need or explicit workflow intent, replay safe prior context, bound session lifetime/concurrency/steps, and package extracted evidence rather than raw browser traces.
- Interactive trace/logging data must be opt-in, privacy-sanitized, and disabled for zero-retention or private contexts; trace metadata is diagnostic context, not citable evidence.

## Candidate Assimilation Targets

1. Search-result scrape merge: make our web retrieval treat thin SERP rows as candidates for page fetch/enrichment. Implemented first pass.
2. Provider fallback semantics: keep provider failure internal and return structured quality metadata rather than chat-visible provider apologies.
3. Engine waterfall model: represent fetch/read/browser/index options as policy-driven capabilities with budgets and stop conditions.
4. URL filtering and blocklist hygiene: filter obvious blocked/junk/internal URLs before spending fetch budget. Implemented first pass for page extraction candidates.
5. Page-to-document extraction: preserve title, URL, metadata, status, markdown/text, and quality flags as an evidence item. Implemented binary/unsupported-content rejection first pass for web fetch.
6. E2E-style evals: add tests that exercise search plus enrichment rather than only isolated fixture snippets.
7. Route unsupported PDF/document content into admitted document/PDF extraction lanes rather than merely rejecting it at web-fetch time. Implemented first pass for PDF page-extraction candidates by feeding `unsupported_content_type:application/pdf` through the existing PDF extraction lane.
8. Iterative gap loop: for broad/current/comparative research, run discovery, read/fetch promising candidates, derive gaps from evidence, issue bounded follow-up lanes, then synthesize from collected evidence and recorded gaps. Implemented CD-level policy update; runtime execution remains to verify.
9. Parse-only document lane: uploaded or fetched document-like artifacts should reject browsing/rendering options, bypass normal web cache unless explicit, and emit normalized document evidence. Implemented CD-level policy update plus PDF fetch handoff; office-document runtime extraction remains future work.
10. Site corpus pack: when the target is a site/docs set/URL collection, map a bounded URL set, batch-read pages, expose compact page rows and full-text evidence refs, and reuse fresh larger cache entries for smaller limits when privacy policy permits. Implemented CD-level policy update; runtime execution remains future work.
11. Async map/crawl status projection: for site-scale research, separate discovery, crawl/read execution, and bounded result windows; make final synthesis consume completed page evidence refs, not raw handles or queue status. Implemented CD-level policy update; runtime execution remains future work.
12. High-volume candidate filtering: when discovery is sparse or broad research needs coverage, broaden once, keep a capped candidate pool, rerank/filter before fetch, and record selected/rejected candidate traces. Implemented CD-level policy update; runtime execution remains future work.
13. Retrieval execution lifecycle: high-volume retrieval needs owner/run grouping, bounded backlog, lock ownership, drift reconciliation, and internal-only queue metrics before completed page refs become evidence. Implemented CD-level policy update; runtime execution remains future work.
14. Managed search-index lane: use admitted indexes as optional candidate discovery with hybrid/keyword/semantic/BM25 modes, quality/freshness/rank metadata, and empty-candidate failure behavior. Implemented CD-level policy update; runtime execution remains future work.
15. Terminal failure projection: convert dead-lettered, crashed, expired, or cancelled retrieval/extraction work into bounded failed artifacts with sanitized gap reasons instead of retries or silent missing output. Implemented CD-level policy update; runtime execution remains future work.
16. Dynamic interaction lane: use bounded browser sessions only when static/reader/rendered retrieval is insufficient, replay retained context safely, run snapshot/action loops with step and timeout limits, and expose only extracted evidence refs. Implemented CD-level policy update; runtime execution remains future work.

## Remaining Work

- Continue parsing unreviewed crawl/map compatibility controllers, especially V1/V2 cancel/error/status websocket variants not yet covered.
- Continue parsing batch scrape, extract, browser tests/SDK surfaces, and remaining agent support files for reusable async/batch/result-projection patterns.
- Continue parsing remaining scraper utility tests and queue/worker internals for retry, concurrency, idempotency, and cleanup behavior.
- Continue parsing remaining native/TS parser tests for non-PDF document extraction and structured-artifact stability.
- Keep scanning remaining files for any general web-tooling primitive that improves discovery quality, extraction quality, evidence packing, lifecycle bounds, or retry safety.
