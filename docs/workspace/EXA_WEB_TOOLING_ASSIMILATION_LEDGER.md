# Exa Web Tooling Assimilation Ledger

Created: 2026-05-10

Source repo: `https://github.com/exa-labs/exa-mcp-server`

Local source clone: `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server`

Source revision: `0cfbeed`

License: MIT

## Goal

Track Exa pattern assimilation for improving web research retrieval and evidence packaging.

This ledger is intentionally about portable patterns, not copied source, provider-specific routes, fixed answer formats, or model/tool hardcoding.

## Guardrails

- Prefer CD/policy-owned behavior over Rust application hardcoding.
- Do not make Exa the only web path.
- Do not copy Exa's prompt examples, provider-specific category syntax, or deprecated tool routing as user-facing workflow behavior.
- Preserve user and agent intent in visible request payloads; no hidden query expansion.
- Treat advanced provider controls as optional capability fields, not default behavior.
- Keep raw provider payloads, diagnostics, costs, and statuses hidden until synthesized or projected as evidence metadata.
- Track parsed source surfaces here so future passes can continue without repeating or missing coverage.

## Assimilation Targets

| ID | Target | Why it matters | Status |
| --- | --- | --- | --- |
| `EXA-PATTERN-001` | Semantic query planning | Exa emphasizes describing the ideal source/page, not keyword stuffing. Useful for our query-pack policy without hardcoding domains. | implemented |
| `EXA-PATTERN-002` | Query diversity by angle | Exa warns that synonym-only query packs waste budget. Useful for broad/comparative research lanes. | implemented |
| `EXA-PATTERN-003` | Search then fetch escalation | Default Exa tools separate search/highlights from full URL fetch. Useful for deciding when snippets are enough and when page fetch is needed. | implemented |
| `EXA-PATTERN-004` | Provider result envelope preservation | Exa retains `requestId`, resolved search type, scores, highlights, highlight scores, statuses, subpages, search time, and cost. Useful as hidden evidence metadata. | implemented |
| `EXA-PATTERN-005` | Optional advanced controls | Advanced search exposes source-class/category, date, include/exclude, additional query, summary/highlight, live crawl, and subpage controls as request fields. Useful as Tool CD capability vocabulary. | implemented |
| `EXA-PATTERN-006` | Per-URL status/error retention | Fetch responses can mix successful pages with per-URL errors. Useful for partial-failure synthesis and hard-vs-soft retrieval classification. | implemented |
| `EXA-PATTERN-007` | Sanitized provider payloads | Sanitizer recursively strips sensitive keys while retaining safe evidence fields. Useful for raw-payload quarantine and evidence metadata shaping. | implemented |
| `EXA-PATTERN-008` | Source quality and validation | Exa skill docs treat search results as similarity candidates that still require validation, filtering, dedupe, and quality weighting. | implemented |
| `EXA-PATTERN-009` | Async deep research caution | Exa's deep research/check tools expose handle/status/report shape, but are deprecated in favor of advanced search. Useful as lifecycle pattern only. | implemented |
| `EXA-PATTERN-010` | Tool catalog with default/advanced split | Exa defaults simple search/fetch and keeps advanced/deprecated tools disabled unless configured. Useful for Tooling CD admission. | implemented |

## Parsed This Pass

| Path | Status | Relevant pattern | Result |
| --- | --- | --- | --- |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/LICENSE` | parsed | license boundary | Confirmed MIT license; still assimilating patterns rather than source. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/package.json` | parsed | tool purpose and dependencies | Logged search/crawl MCP purpose, configurable tool selection, result count controls, live crawling, and Exa SDK boundary. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/server.json` | parsed | MCP package/admission metadata | Extracted hosted remote declaration, package identity, default tool subset, and API-key transport as capability metadata. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/mcp-handler.ts` | parsed | tool catalog and default split | Extracted default enabled search/fetch, optional advanced search, deprecated specialty tools, user-provided-key gate for deep search, and non-fatal analytics setup. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/webSearch.ts` | parsed | semantic search and highlights | Extracted simple search request shape, natural-language query guidance, optional category hint, highlights-only search, search time metadata, and search-then-fetch guidance. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/webSearchAdvanced.ts` | parsed | advanced search capability surface | Extracted optional filters, date ranges, include/exclude text/domains, additional queries, text/context/summary/highlight controls, live crawl max age/timeout, and subpage crawling as request-level knobs. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/webFetch.ts` | parsed | URL fetch and partial failures | Extracted batch URL fetch, text max characters, per-URL status/error handling, mixed success/error output, and status metadata. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/deepSearch.ts` | parsed | query expansion and grounded output | Logged automatic query expansion, additional queries, citations/grounding, structured output, and provider-owned synthesis. Decision: do not copy provider synthesis or output format into our workflow. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/deepResearchStart.ts` and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/deepResearchCheck.ts` | parsed | async handle/status lifecycle | Extracted start/check/status/report pattern and polling caution. Decision: lifecycle useful, provider model/tool behavior not assimilated. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/types.ts` | parsed | provider envelope fields | Extracted safe result fields: score, highlights, highlight scores, summary, text, entities, extras links/images, subpages, statuses, grounding, search time, and cost. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/utils/exaResponseSanitizer.ts` | parsed | result sanitization | Extracted recursive sensitive-key stripping and allowlisted result/status/grounding fields. Pattern maps to evidence metadata, not chat-visible raw payloads. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/utils/errorHandler.ts` | parsed | retry and rate-limit handling | Extracted transient retry lane, rate-limit classification, and user-fix error surface. Pattern maps to tool-result quality lanes and permission/auth status. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/SKILL.md` | parsed | orchestrated research process | Extracted query assessment, broad search, extraction, filtering, dedupe, ranking, synthesis, and gap follow-up as process ideas. Rejected subagent/model/output-format instructions as non-portable. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/searching.md` | parsed | semantic query writing | Extracted target-page query phrasing, angle diversity, time encoding, result count sizing, and search-result validation. Rejected provider-specific inline syntax as default behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/filtering.md` | parsed | hard/soft filter order | Extracted hard-filter-first, soft-filter-after-read, temporal boundary handling, and precision-vs-completeness tradeoff. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/source-quality.md` | parsed | source quality weighting | Extracted practitioner-vs-commentator weighting, misaligned incentive detection, source convergence, and quality tags. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/synthesis.md` | parsed | synthesis guidance | Extracted lead-with-answer, theme-over-source, disagreement, confidence signals, and citation practice. Decision: useful as quality guidance, not forced output format. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/extraction.md` | parsed | snippet-vs-fetch decision | Extracted when snippets are enough, when to full-fetch, batching known URLs, missing-data distinctions, and evidence confidence labels. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/skills/search/references/patterns-*.md` | surveyed | domain examples | Parsed people, companies, papers, news, code, and relationship examples as source-class/query-shape examples only. Do not copy as hardcoded routes or golden-case candidates. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/utils/exaResponseSanitizer.test.ts` | parsed | sanitizer proof | Confirmed sensitive-key removal, result allowlisting, status retention, and grounding/citation retention. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/tools/webSearch.test.ts` | parsed | search request proof | Confirmed category stripping, configured search type, highlights request, session header, and empty-result path. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/tools/webFetch.test.ts` | parsed | fetch request proof | Confirmed content request shape and mixed success/error behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/fixtures/exaResponses.ts` | parsed | response fixtures | Confirmed minimal response envelopes for search, empty search, contents success, and contents error. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/README.md` and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/npm.readme.md` | parsed | catalog, skill, and install docs | Extracted default/off-by-default/deprecated tool catalog, dynamic result sizing, query variation, category restriction cautions, and token-isolation ideas. Rejected provider-specific tool restrictions and output formats. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/llm_mcp_docs.txt` | surveyed | MCP protocol reference bundle | Treated as bundled upstream MCP reference, not Exa implementation. Retained only general MCP ideas already covered by Tool CD admission/transport policy. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/gemini-extension.json`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/env.example`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/server.json` | parsed | package/admission metadata | Confirmed hosted/stdio packaging, API-key/env configuration, default tool subset, and optional tool selection. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/config.ts` | parsed | request metadata | Extracted integration/session headers and default result/character limits as adapter metadata patterns. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/exaCode.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/peopleSearch.ts`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/tools/linkedInSearch.ts` | parsed | deprecated specialty routes | Confirmed these mostly alias general search/advanced search patterns. Decision: assimilate as source-class hints/admission metadata, not separate hardcoded workflow routes. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/utils/auth.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/api/mcp.ts`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/api/mcp-oauth.ts` | parsed | auth/rate/transport lifecycle | Extracted request config priority, per-request handler isolation, credential stripping before transport/analytics, rate-limit-only-on-tool-call, OAuth/API-key/free-tier classification, and CORS/error wrapping. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/api/well-known-mcp-config.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/api/well-known-oauth-protected-resource.ts`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/api/well-known-openai-apps-challenge.ts` | parsed | discovery and verification endpoints | Extracted capability discovery schema and auth metadata as Tool CD admission patterns. No direct retrieval-quality pattern. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/stdio.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/src/stdio-cli.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/stdio.test.ts`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/api/mcp.test.ts` | parsed | transport/config proofs | Confirmed env-derived config, user-provided-key classification, session id propagation, credential stripping, OAuth challenge, and rate-limit response behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/mcp-handler.test.ts`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/tools/config.test.ts`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tests/unit/utils/errorHandler.test.ts` | parsed | catalog/header/error proofs | Confirmed default tool list, explicit tool selection, deprecated-alias behavior, session/integration headers, free-tier rate-limit guidance, transient retries, and non-transient no-retry behavior. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/Dockerfile`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/vercel.json`, `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/tsconfig*.json`, and `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/vitest.config.ts` | parsed | build/deploy/test configuration | No retrieval mechanism beyond bounded function duration/memory and ordinary build/test setup. No assimilation action. |
| `/Users/jay/.openclaw/workspace/local/workspace/shadow/external-repos/exa-mcp-server/package-lock.json` | skipped-generated | dependency lockfile | Generated dependency inventory; no portable retrieval pattern needed for this pass. |

## Assimilation Decisions

| Decision | Status | Target | Notes |
| --- | --- | --- | --- |
| Add semantic query planning vocabulary | accepted | Batch-query policy and Tool CD | Use general rules: describe target sources, diversify by angle/source class/time/aspect, avoid synonym-only packs, and record query intent. |
| Preserve provider result envelopes | accepted | Evidence pack and Tool CD | Keep scores, highlights, highlight scores, statuses, subpages, resolved search type, search time, cost, and request ID as hidden metadata where available. |
| Treat search highlights as candidates, not proof | accepted | Batch-query policy | Highlight/snippet rows can start extraction, but soft claims still need validation or low-evidence framing. |
| Use fetch escalation from evidence state | accepted | Page extraction policy | Fetch when highlights are thin, a source is promising, a soft filter requires body context, or requested fields are missing. |
| Keep advanced provider controls optional | accepted | Tool CD | Categories/source class, date ranges, additional query sets, text/context/summary/highlight controls, live freshness, and subpage crawl belong in capability metadata. |
| Keep source-quality tags as evidence metadata | accepted | Research-plane policy | Practitioner/commentator/vendor/noise tags can influence ranking without forcing final answer structure. |
| Reject Exa-specific tool routing as default behavior | accepted | Ledger only | Inline `category:` syntax, deprecated tools, hosted MCP URL parameters, and provider model strings are not portable workflow behavior. |
| Reject provider-owned synthesis as replacement for our synthesis step | accepted | Ledger only | Exa deep search/deep research can return answers, but our workflow still requires ToolObservation -> Evidence -> Synthesis -> FinalResponse. |
| Separate protocol success from evidence usefulness | accepted | Batch-query policy, Research-plane policy, Tool CD | Exa has empty-result and error-status handling. Our policy now classifies usable, low-signal, irrelevant, empty, partial, bad-shape, auth, rate-limit, transient, and permanent failure states. |
| Collapse specialty tools into general capability metadata | accepted | Tool CD | Code, people, LinkedIn, company, and research-paper surfaces are useful as source-class examples, but not as hardcoded workflow branches. |
| Keep transport/auth lessons out of final chat | accepted | Tool CD | Credentials, raw provider errors, rate-limit internals, and diagnostic status stay in telemetry/evidence metadata unless synthesized for the user. |

## Implemented This Pass

| Change | File | Pattern |
| --- | --- | --- |
| Added semantic query-planning and query-diversity policy vocabulary for target-source descriptions, angle diversity, and visible query-pack reasoning. | `client/runtime/config/batch_query_policy.json`, `client/runtime/config/research_plane_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `EXA-PATTERN-001`, `EXA-PATTERN-002`, `EXA-PATTERN-008` |
| Added provider envelope and search-result metadata fields for scores, highlights, highlight scores, statuses, subpages, resolved search type, search time, cost, and request IDs. | `client/runtime/config/batch_query_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `EXA-PATTERN-004`, `EXA-PATTERN-006`, `EXA-PATTERN-007` |
| Added search-highlight-to-fetch escalation guidance without forcing full fetch for every result. | `client/runtime/config/batch_query_policy.json`, `client/runtime/config/research_plane_policy.json` | `EXA-PATTERN-003`, `EXA-PATTERN-005` |
| Added optional advanced-search capability vocabulary while keeping provider-specific controls adapter-owned. | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `EXA-PATTERN-005`, `EXA-PATTERN-010` |
| Added provider request lifecycle and retrieval-status classification so hard provider/runtime failures and soft low-evidence failures are not conflated. | `client/runtime/config/batch_query_policy.json`, `client/runtime/config/research_plane_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `EXA-PATTERN-006`, `EXA-PATTERN-007`, `EXA-PATTERN-010` |
| Added tool admission contract for default, optional, deprecated/provider-specific, and permission-gated capabilities without creating provider-specific workflow routes. | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | `EXA-PATTERN-010` |

## Remaining Assimilation Estimate

Current target: extract portable web-retrieval patterns from Exa MCP and map them into our policy/CD surfaces without provider lock-in.

| Work class | Remaining | Status | Notes |
| --- | ---: | --- | --- |
| High-priority retrieval-quality patterns | 0% | complete | Semantic query planning, angle diversity, search-then-fetch escalation, provider envelopes, source-quality signals, partial failures, sanitization, lifecycle status, and tool admission are all represented. |
| Repo surface parsing for this target | 0% | complete | Pattern-bearing source, docs, tests, transport, auth, package metadata, and deploy/config files have been reviewed or explicitly skipped as generated/no-op. |
| Optional provider-adapter implementation | not part of this pass | deferred | A future Exa adapter could implement these capability fields, but this ledger intentionally did not make Exa a required runtime provider. |
| Optional transport/auth follow-up | not part of this pass | deferred | Revisit only if a concrete MCP admission, OAuth, or rate-limit blocker appears. |
| Workflow/golden impact measurement | separate validation task | open | Needs a fresh workflow/golden run after these CD/policy changes are committed or selected for testing. |

Practical remaining assimilation from Exa itself is done. The next useful work is validation against the research workflow, not another Exa parsing pass.

## Integration Map

This section tracks whether assimilated patterns have an operational home instead of remaining only ledger notes.

| Pattern | Primary home | Secondary home | Integration status |
| --- | --- | --- | --- |
| Semantic query planning and query diversity | `client/runtime/config/batch_query_policy.json:batch_query.semantic_query_planning` | `client/runtime/config/research_plane_policy.json:adaptive_retrieval.semantic_query_planning`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:progressive_retrieval_contract.semantic_query_planning` | wired |
| Search-highlight-to-fetch escalation | `client/runtime/config/batch_query_policy.json:batch_query.page_extraction.search_result_followup` | `client/runtime/config/research_plane_policy.json:evidence_extraction.search_result_followup`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:structured_result_extraction.search_then_fetch_escalation` | wired |
| Provider result envelope and sanitized metadata | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:provider_result_envelope` | `client/runtime/config/batch_query_policy.json:batch_query.provider_result_envelope`, `client/runtime/config/research_plane_policy.json:evidence_extraction.provider_result_envelope` | wired |
| Source quality and validation signals | `client/runtime/config/research_plane_policy.json:evidence_extraction.source_quality_signals` | `client/runtime/config/batch_query_policy.json:batch_query.semantic_query_planning`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:provider_result_envelope.non_goals` | wired |
| Request lifecycle and hard/soft status separation | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:provider_error_contract` | `client/runtime/config/batch_query_policy.json:batch_query.provider_request_lifecycle`, `client/runtime/config/research_plane_policy.json:evidence_extraction.retrieval_status_classification` | wired |
| Default/advanced/deprecated capability admission | `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json:tool_admission_contract` | `client/runtime/config/batch_query_policy.json:batch_query.advanced_search_controls` | wired |
| Async high-volume retrieval lifecycle | existing `async_result_lifecycle` blocks | Exa deep-research pattern recorded as lifecycle only, not provider-owned synthesis | already covered |

Provider-specific names in `source_pattern` are provenance labels only. The policy/CD blocks now mark `provider_binding` as `none_source_pattern_is_attribution_only`, and selection remains based on user intent, evidence state, capability availability, and tool-result quality.

## Repo Coverage Status

| Area | Paths | Priority | Reason |
| --- | --- | --- | --- |
| Pattern-bearing retrieval/docs/tests | `README.md`, `npm.readme.md`, `skills/**`, `src/tools/**`, `src/types.ts`, `src/utils/**`, relevant `tests/**` | complete for current pass | Portable retrieval, evidence, source-quality, status, and catalog patterns have been recorded and mapped to policy/CD changes. |
| Transport/auth/deploy surfaces | `api/**`, `src/stdio*.ts`, `server.json`, `gemini-extension.json`, `env.example`, `vercel.json`, `Dockerfile` | complete for current pass | Useful patterns were assimilated as capability discovery, auth/status, request isolation, and sanitization metadata. |
| Generated/dependency/build-only surfaces | `package-lock.json`, `tsconfig*.json`, `vitest.config.ts` | intentionally skipped or no-op | No portable retrieval behavior to assimilate beyond ordinary build/test configuration. |

No remaining Exa source surface is known to contain high-priority retrieval-quality material for this pass. Future work should only return here if a concrete gap points at OAuth/admission, MCP transport, or provider-specific adapter implementation.
