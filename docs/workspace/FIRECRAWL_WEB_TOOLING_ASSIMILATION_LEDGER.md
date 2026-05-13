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
- Parsed: 62
- Not parsed: 1216
- Skipped generated: 11
- Skipped media or sample: 68

## Parsed Files

| File | Why Parsed | Useful Pattern Signal |
| --- | --- | --- |
| `README.md` | Product-level feature surface. | Search can optionally return full page content, not just search snippets; outputs should be LLM-ready markdown/structured data. |
| `SELF_HOST.md` | Deployment/provider constraints. | Provider stack is environment-dependent; self-hosted modes may lack advanced anti-bot/fire-engine capabilities and need explicit fallback expectations. |
| `CLAUDE.md` | Maintainer workflow and tests. | Firecrawl prefers E2E/snips tests with happy and failure paths around actual API behavior. |
| `apps/api/src/controllers/v2/search.ts` | Public search request entrypoint. | Normalize/validate request, preserve agent interop and ZDR boundaries, then delegate to a single search executor. |
| `apps/api/src/search/execute.ts` | Search orchestration. | Overfetch search results, categorize domains, optionally scrape selected candidates, then merge scraped content back into the search response. |
| `apps/api/src/search/scrape.ts` | Search-result enrichment. | Treat SERP rows as candidates; filter blocked URLs, directly scrape result URLs concurrently, and merge richer documents into original result rows. |
| `apps/api/src/search/v2/index.ts` | Provider fallback. | Try configured premium engine first, then self-hosted SearXNG, then DuckDuckGo, failing to an empty response rather than leaking provider errors. |
| `apps/api/src/scraper/scrapeURL/index.ts` | Scrape pipeline entrypoint. | Build feature flags from requested output, choose engine waterfall, use prefetch/robots/transformers/postprocessors as a page-to-document pipeline. |
| `apps/api/src/search/v2/ddgsearch.ts` | DuckDuckGo provider implementation. | HTML endpoint pagination, seen-URL dedupe, anti-bot retry, timeout handling, and user-agent rotation are provider-internal mechanics. |
| `apps/api/src/search/v2/searxng.ts` | SearXNG provider implementation. | Fetch enough pages to satisfy requested result count, stop on empty pages, normalize provider rows into a shared web result shape. |
| `apps/api/src/search/v2/fireEngine-v2.ts` | Premium search provider implementation. | Provider calls are retried behind an adapter boundary and can return partial source-type coverage. |
| `apps/api/src/lib/search-query-builder.ts` | Structured query filter builder. | Categories, include domains, exclude domains, and PDF filters compile into query/filter lanes while preserving a category map for result metadata. |
| `apps/api/src/controllers/v2/types.ts` | Request schema and search options. | Strict request schema supports sources, categories, domains, scrape options, timeout bounds, and format defaults as typed input rather than prompt phrasing. |
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

## Candidate Assimilation Targets

1. Search-result scrape merge: make our web retrieval treat thin SERP rows as candidates for page fetch/enrichment. Implemented first pass.
2. Provider fallback semantics: keep provider failure internal and return structured quality metadata rather than chat-visible provider apologies.
3. Engine waterfall model: represent fetch/read/browser/index options as policy-driven capabilities with budgets and stop conditions.
4. URL filtering and blocklist hygiene: filter obvious blocked/junk/internal URLs before spending fetch budget. Implemented first pass for page extraction candidates.
5. Page-to-document extraction: preserve title, URL, metadata, status, markdown/text, and quality flags as an evidence item. Implemented binary/unsupported-content rejection first pass for web fetch.
6. E2E-style evals: add tests that exercise search plus enrichment rather than only isolated fixture snippets.
7. Future target: route unsupported PDF/document content into admitted document/PDF extraction lanes rather than merely rejecting it at web-fetch time.

## Remaining Work

- Parse the provider implementations under `apps/api/src/search/v2/*`.
- Parse request schema/types under `apps/api/src/controllers/v2/types.ts`.
- Parse scrape engines under `apps/api/src/scraper/scrapeURL/engines*`.
- Parse HTML/markdown conversion and metadata extraction in both TS and native Rust paths.
- Continue parsing crawler/map URL filtering, sitemap, robots, and blocklist utilities; `crawler.ts`, `map-utils.ts`, `canonical-url.ts`, and `robots-txt.ts` are now parsed, but tests and utility helpers remain.
- Parse relevant snips/e2e tests for behavior expectations and failure modes.
