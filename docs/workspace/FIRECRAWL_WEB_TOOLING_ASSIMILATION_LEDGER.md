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
- Parsed: 104
- Not parsed: 1174
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

## Remaining Work

- Parse the provider implementations under `apps/api/src/search/v2/*`.
- Parse request schema/types under `apps/api/src/controllers/v2/types.ts`.
- Parse scrape engines under `apps/api/src/scraper/scrapeURL/engines*`.
- Continue parsing HTML/markdown conversion and metadata extraction in both TS and native Rust paths; native document conversion is now parsed, but TS document/upload controller plumbing remains.
- Continue parsing crawler/map URL filtering, sitemap, robots, and blocklist utilities; `crawler.ts`, `map-utils.ts`, `canonical-url.ts`, and `robots-txt.ts` are now parsed, but tests and utility helpers remain.
- Continue parsing deep-research queue/worker/logging paths if we choose to assimilate async job lifecycle beyond the state/loop/controller surface already parsed.
- Parse relevant snips/e2e tests for behavior expectations and failure modes.
