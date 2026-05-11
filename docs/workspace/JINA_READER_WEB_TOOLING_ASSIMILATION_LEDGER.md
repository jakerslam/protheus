# Jina Reader Web Tooling Assimilation Ledger

Status: first stable pattern pass complete
Target repo: `jina-ai/reader`
Local clone: `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/jina-reader`
Source revision parsed: `bc0c608`
Assimilation rule: extract portable retrieval and evidence-shaping patterns only. Do not copy provider-specific routing, SaaS billing, auth gates, model IDs, or user-visible answer formats.

## Assimilation Target

Improve our web retrieval primitive for real research questions by making search results more processible before synthesis:

- Search rows should be treated as candidates, not the final evidence packet.
- Promising result URLs should be converted into reader-friendly page artifacts when snippets are not enough.
- The tool should retain safe metadata, readiness state, cache state, link/image summaries, and chunk references for synthesis.
- The agent should still choose strategy from the user request and evidence state. No hardcoded domains, prompt phrases, topic candidates, or final-answer structures.

## Repo Coverage

| Area | Files Parsed | Status | Pattern Disposition |
| --- | --- | --- | --- |
| Public behavior and API model | `README.md`, `architecture.md`, `package.json` | Complete | Assimilated conceptually: search-plus-read, response variants, selector/readiness controls, cache privacy, streaming completeness. |
| Request/option contract | `src/dto/crawler-options.ts` | Complete | Assimilated as general request metadata: representation, readiness timing, selector targeting, token budget, cache rules, link/image retention. |
| Search path | `src/api/searcher.ts` | Complete | Assimilated search rows followed by top-result content reads, partial result qualification, provider fallback, local cache fallback. |
| Crawl/read path | `src/api/crawler.ts` | Complete | Assimilated static-first, dynamic escalation, stale-cache/side-load fallback, early-vs-final snapshot acceptance, binary/document handling. |
| Markdown/page formatter | `src/services/snapshot-formatter.ts`, `src/services/markify.ts` | Complete | Assimilated formatted page envelope, link/image summaries, heading chunking, poor-transform fallback, base URL handling. |
| Binary and PDF extraction | `src/services/binary-extractor.ts`, `src/services/pdf-extract.ts` | Complete | Assimilated multi-document extraction pattern: PDF/office/image/XML/text with page-level artifacts and metadata. |
| Static fetch/browser support | `src/services/curl.ts`, `src/services/puppeteer.ts` | Pattern skim complete | Assimilated static fetch impersonation and browser escalation as capability patterns, not implementation syntax. |
| Test coverage map | `tests/e2e/*`, `tests/unit/*` inventory and targeted search | Complete enough for pattern pass | Assimilated test themes: cache controls, response variants, selectors, chunking, links/images, token budgets, base URL resolution, streaming. |

## Patterns Accepted

1. Search-plus-read as a primitive
   - Pattern: search returns candidate rows, then selected top URLs are read into content-bearing artifacts.
   - Integration target: `batch_query_policy.page_extraction.reader_page_conversion`, `web_retrieval_v0.tool_cd.retrieval_primitives.search_plus_read`.
   - Why: addresses low-value SERP-only outputs without requiring domain-specific queries.

2. Response representation negotiation
   - Pattern: page evidence can be markdown, content, text, html excerpt, screenshot/page image ref, or structured summaries.
   - Integration target: `reader_page_conversion.representation_variants`, evidence content variants.
   - Why: lets synthesis receive the right artifact type without prescribing final answer format.

3. Readiness and selector controls
   - Pattern: choose early or final snapshots by readiness state; use wait/target/remove selectors when content structure demands it.
   - Integration target: `reader_page_conversion.readiness_controls` and `selector_controls`.
   - Why: makes dynamic or noisy pages diagnosable without forcing browser mode for all retrieval.

4. Cache privacy and lifecycle admission
   - Pattern: do not reuse or persist cache for private/cookie/script/viewport/instructional requests; record cache freshness.
   - Integration target: `reader_page_conversion.cache_admission`, existing cleanup-bound cache policy.
   - Why: keeps cache useful for tests and users without turning it into hidden stale evidence.

5. Link/image summary retention
   - Pattern: links and images can be retained as summaries with refs, alt text, and safe URL normalization.
   - Integration target: provider/result envelope fields and structured artifact kinds.
   - Why: gives the LLM non-prose evidence without raw payload leakage.

6. Heading-aware chunking with context
   - Pattern: split markdown by heading path and optionally include parent headings in each chunk.
   - Integration target: evidence chunking strategy and content variants.
   - Why: improves large-page synthesis and reduces single-snippet overconfidence.

7. Multi-document page artifacts
   - Pattern: PDFs, office docs, images, XML, and text files are normalized to page/document artifacts with metadata.
   - Integration target: document extraction handled types and evidence variants.
   - Why: broadens web retrieval beyond HTML and PDF without provider lock-in.

8. Partial failure and fallback semantics
   - Pattern: retain successful URL reads, record per-URL errors, and distinguish stale cache/side-load fallback from fresh evidence.
   - Integration target: existing provider request lifecycle and retrieval status classification.
   - Why: prevents hard failures from looking like empty successful evidence.

## Patterns Rejected Or Deferred

| Pattern | Decision | Reason |
| --- | --- | --- |
| Jina endpoint names (`r.jina.ai`, `s.jina.ai`) as workflow routes | Rejected | Provider-specific and would hardcode user-facing behavior. |
| Vendor model names for alt text / ReaderLM / VLM | Rejected | Model selection must remain adapter/config owned, not hardcoded into workflow. |
| SaaS auth, billing, charge amounts, tier checks | Rejected | Not relevant to our primitive contract. |
| Exact curl-impersonate fingerprint values | Rejected | Syntax/provider implementation detail, not a general CD pattern. |
| Provider-specific SERP order | Rejected | Useful as adapter behavior only; workflow should reason over result quality and coverage. |
| User-visible output templates from Reader examples | Rejected | The final answer format remains LLM/user-intent driven. |

## Integration Targets Updated

- `client/runtime/config/batch_query_policy.json`
  - Added reader page-conversion policy, representation/readiness/selector/cache/link-image/chunking controls, broader document artifact types.
- `client/runtime/config/research_plane_policy.json`
  - Added reader content artifact handling for synthesis-visible evidence, not chat-visible raw payload.
- `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
  - Added search-plus-read and reader representation primitives to the Tool CD.

## Follow-Up Candidates

- Add a small deterministic test fixture that proves a search row can be followed by a selected page-read artifact before synthesis.
- Add measurement fields for `reader_artifact_count`, `read_success_count`, and `read_partial_failure_count` in live golden reports if the current telemetry does not already expose them.
- If golden failures remain low-data, compare real web runs with and without search-plus-read enabled to see whether the remaining gap is retrieval volume or synthesis discipline.
