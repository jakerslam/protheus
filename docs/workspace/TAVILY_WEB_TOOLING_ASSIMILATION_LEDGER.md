# Tavily Web Tooling Assimilation Ledger

Created: 2026-05-10

Source repo: `https://github.com/tavily-ai/tavily-mcp`

Local source clone: `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp`

Source revision: `7bcf907`

License: MIT

## Goal

Track Tavily pattern assimilation for improving web retrieval as a general primitive.

This ledger is intentionally about portable patterns, not copied source, provider-specific routing, fixed answer formats, or model/tool hardcoding.

## Guardrails

- Prefer CD/policy-owned behavior over Rust application hardcoding.
- Do not make Tavily the only web path or a privileged workflow route.
- Do not copy Tavily's tool names into user intent classification.
- Treat provider-owned answers as evidence candidates, not final assistant answers.
- Keep domain, time, exact-match, depth, map, crawl, and extraction controls explicit in tool/policy metadata.
- Preserve provider scores, dates, favicons, images, and raw-content availability as hidden evidence metadata until synthesized.
- Track parsed source surfaces here so future passes can continue without repeating or missing coverage.

## Assimilation Targets

| ID | Target | Why it matters | Status |
| --- | --- | --- | --- |
| `TAVILY-PATTERN-001` | Search/extract/map/crawl capability split | Tavily separates discovery, page extraction, site mapping, and crawling. Useful for Tooling CD capability vocabulary without forcing one route. | implemented |
| `TAVILY-PATTERN-002` | Provider depth and latency profiles | Search depth includes basic, advanced, fast, and ultra-fast. Useful as adapter-owned depth metadata rather than model hardcoding. | implemented |
| `TAVILY-PATTERN-003` | Time, domain, country, and exact-match filters | Tavily exposes filters that should remain explicit request fields when user intent or evidence state requires them. | implemented |
| `TAVILY-PATTERN-004` | Optional raw content and query-reranked extraction | Search can request raw content, and extract can rerank chunks by a query. Useful for search-then-fetch and chunk-ranked evidence. | implemented |
| `TAVILY-PATTERN-005` | Site map before crawl | Map returns URL structure without page content; crawl returns content with depth/breadth/limit controls. Useful for high-volume discovery before expensive extraction. | implemented |
| `TAVILY-PATTERN-006` | Natural-language crawl instructions plus path/domain filters | Crawl/map can receive instructions and select path/domain regexes. Useful as explicit discovery policy, not prompt-specific hardcoding. | implemented |
| `TAVILY-PATTERN-007` | Provider result metadata | Search rows expose score, published date, raw content, favicon, images, and follow-up questions. Useful for evidence metadata and gap signals. | implemented |
| `TAVILY-PATTERN-008` | Default-parameter overlay and empty-value pruning | Tavily overlays defaults and strips empty values before calls. Useful as adapter request hygiene, not hidden query rewriting. | implemented |
| `TAVILY-PATTERN-009` | Async research polling lifecycle | Tavily research returns a request id and polls with bounded backoff/timeouts. Useful as lifecycle vocabulary for high-volume retrieval. | implemented |
| `TAVILY-PATTERN-010` | Structured provider errors with docs refs | Auth/rate-limit/provider errors include status and documentation refs. Useful for hard-vs-soft retrieval classification without leaking raw error bodies. | implemented |

## Parsed This Pass

| Path | Status | Relevant pattern | Result |
| --- | --- | --- | --- |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/README.md` | parsed | tool catalog and auth/default parameter docs | Extracted search, extract, map, crawl, remote/local MCP, API-key/OAuth setup, default parameter overlay, and optional human-id tracking. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/package.json` | parsed | package/admission metadata | Confirmed MIT license, MCP package identity, Node/TypeScript implementation, and crawl/search/extract keywords. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/src/index.ts` | parsed | implementation surface | Extracted tool schemas, search/extract/crawl/map/research API boundaries, default parameter overlay, request cleanup, result formatting, 401/429 handling, docs refs, session/human headers, and async research polling. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/tsconfig.json`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/Dockerfile`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/smithery.yaml` | surveyed | build/deploy metadata | No retrieval primitive beyond package/runtime admission. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/package-lock.json` | skipped-generated | dependency lockfile | Generated dependency inventory; no portable retrieval behavior needed for this pass. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/tavily-mcp/assets/**` | skipped-media | documentation screenshots/media | Static screenshots and demo media; no implementation pattern. |

## Assimilation Decisions

| Decision | Status | Target | Notes |
| --- | --- | --- | --- |
| Keep search, extract, map, crawl, and research as capability vocabulary | accepted | Tooling CD | These are operations an adapter can declare; they must not become hardcoded Gate 1 or Gate 3 routes. |
| Treat provider answers as candidate evidence | accepted | Evidence policy | Tavily search/research may return answers, but our workflow still requires ToolObservation -> Evidence -> Synthesis -> FinalResponse. |
| Add depth/time/domain/exact-match controls as optional knobs | accepted | Batch-query policy and Tool CD | Controls are used only when user intent or evidence state calls for them and should be visible in request metadata. |
| Add map-before-crawl discovery shape | accepted | Research-plane policy and Tool CD | Site mapping can cheaply discover candidate URLs before content crawl/extract. |
| Preserve scores, dates, favicons, images, follow-up questions, and raw-content availability | accepted | Provider envelope and evidence pack | These help ranking and synthesis calibration; they are not chat-visible raw payloads. |
| Use query-reranked extraction as an evidence-processing primitive | accepted | Evidence pack and chunk-ranking policy | The extraction query can rank chunks without forcing final output format. |
| Treat defaults and empty-value pruning as adapter hygiene | accepted | Tool CD | Defaults may tune adapter behavior, but should not hide query expansion or silently rewrite user intent. |
| Keep async research polling as lifecycle, not provider-owned final answer | accepted | Async result lifecycle | Request IDs, status, polling backoff, timeout, and result refs are useful; provider synthesis is not a replacement for our synthesis step. |

## Implemented This Pass

| Change | File | Pattern |
| --- | --- | --- |
| Added Tavily-derived provider metadata fields for provider answers, follow-up questions, images, favicons, raw-content availability, and published dates. | `orchestration/config/web_research_retrieval_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `TAVILY-PATTERN-007` |
| Added optional search controls for depth profile, exact-match, country/time/domain filters, raw content, image descriptions, and favicon metadata. | `orchestration/config/web_research_retrieval_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `TAVILY-PATTERN-002`, `TAVILY-PATTERN-003`, `TAVILY-PATTERN-004` |
| Added site map and crawl controls as discovery/extraction primitives with bounded depth, breadth, limit, instructions, path/domain selection, and external-link policy. | `orchestration/config/web_research_retrieval_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `TAVILY-PATTERN-001`, `TAVILY-PATTERN-005`, `TAVILY-PATTERN-006` |
| Added adapter request-hygiene and async research polling lifecycle fields without hardcoding Tavily as a runtime provider. | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `TAVILY-PATTERN-008`, `TAVILY-PATTERN-009`, `TAVILY-PATTERN-010` |

## Remaining Assimilation Estimate

| Work class | Remaining | Status | Notes |
| --- | ---: | --- | --- |
| Repo surface parsing for this target | 0% | complete | The repo is compact: README, package metadata, source implementation, and build/deploy files are parsed or explicitly skipped. |
| High-priority portable patterns | 0% | complete | Search/extract/map/crawl split, controls, provider metadata, request hygiene, and async polling are represented. |
| Optional Tavily adapter implementation | not part of this pass | deferred | A future adapter can implement these capability fields if Tavily becomes an admitted provider. |
| Workflow/golden impact measurement | separate validation task | open | This pass changed policy/CD metadata only; run workflow evals when a stable batch is ready. |

## Integration Map

| Pattern | Primary home | Secondary home | Integration status |
| --- | --- | --- | --- |
| Search/extract/map/crawl capability split | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.site_mapping_and_crawl_controls` | `orchestration/config/web_research_retrieval_policy.json:site_mapping_and_crawl` | wired |
| Depth/time/domain/exact-match controls | `orchestration/config/web_research_retrieval_policy.json:advanced_search_controls` | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.advanced_search_controls` | wired |
| Provider result metadata | `orchestration/config/web_research_retrieval_policy.json:provider_result_envelope` | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.provider_result_envelope` | wired |
| Query-reranked extraction | `orchestration/config/web_research_retrieval_policy.json:evidence_pack` | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.retrieval_primitives.chunk_ranked_evidence` | wired |
| Async polling lifecycle | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.async_result_lifecycle` | `orchestration/config/web_research_retrieval_policy.json:async_result_lifecycle` | wired |
| Request hygiene and error docs refs | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:provider_error_contract` | `orchestration/config/web_research_retrieval_policy.json:retrieval_status_classification` | wired |

Provider-specific names in `source_pattern` are provenance labels only. Selection remains based on user intent, evidence state, capability availability, and tool-result quality.

Client runtime note: client/runtime remains compatibility/projection only for this assimilation. The Tavily-derived authority was moved out of client config after checkpoint `a299d49d6`.
