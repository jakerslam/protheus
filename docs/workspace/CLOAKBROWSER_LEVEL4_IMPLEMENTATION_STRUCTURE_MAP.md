# CloakBrowser Level 4 Implementation Structure Map

Created: 2026-05-14

Source assimilation ledger: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`

Level 3 mechanics map: `/Users/jay/.openclaw/workspace/docs/workspace/CLOAKBROWSER_LEVEL3_MECHANICS_ALGORITHM_MAP.md`

Source repo: `https://github.com/CloakHQ/CloakBrowser`

Local clone: `/Users/jay/.openclaw/workspace/local/workspace/assimilations/CloakBrowser-Assimilation/target-repo`

Revision inspected: `6f4f92e`

## Purpose

Level 4 maps the Level 3 mechanics to concrete Infring ownership boundaries: CDs, modules, adapters, artifact stores, and tests. It is still not syntax-level assimilation. The goal is to prevent browser-materialization work from spreading across workflow Rust, shell code, or prompt-specific behavior.

Level 4 should answer:

- Which CD declares the capability?
- Which runtime surface enforces safety?
- Which module normalizes provider results and evidence?
- Which artifact store owns raw payload refs?
- Which tests prove the behavior before live provider work?

## Guardrails

- Keep workflow behavior CD-driven. Runtime code may execute and validate primitives, but should not invent research-specific routing, output formats, or prompt assumptions.
- Keep browser materialization as an admitted web retrieval capability, not the default search path.
- Keep proxy, persistent session, humanized interaction, and service pooling separately admitted. They are not implied by browser materialization.
- Keep raw HTML, screenshots, network traces, console logs, CDP traces, proxy details, and browser args artifact-only unless converted into evidence.
- Keep the agent responsible for query choice and final synthesis. Tooling may expose quality, blocker, and strategy signals.
- Prefer mock-fast contract tests before adding live browser or remote-service dependency tests.

## System Boundary Map

| Boundary | Owns | Must Not Own |
| --- | --- | --- |
| Tool CD | Capability identity, request schema, output schema, budgets, side effects, admission requirements, evidence policy, denied caller controls. | Provider implementation code, hidden workflow decisions, final answer wording. |
| Web conduit | Provider admission, URL safety, redirects, SSRF controls, fetch/browser invocation, provider receipts, raw artifact refs. | User-facing synthesis, workflow routing, prompt-specific research heuristics. |
| Batch query | Candidate collation, provider normalization, ranking, evidence pack rows, quality diagnostics, stop-condition diagnostics. | Browser launch internals, proxy secrets, shell rendering, final answer style. |
| Artifact store | Raw payload refs, retention class, cleanup lifecycle, redaction boundaries. | Chat-visible answer content, evidence truth by itself. |
| Orchestration workflow CD | When to invoke web research, final output contract, retry budget policy, sub-CD composition. | Browser internals, provider credentials, raw tool payload handling. |
| Gateway / Kernel policy | Authority, permissions, external membrane, secret boundaries, safety enforcement. | Research answer formatting, provider ranking details. |
| Assurance / evals | Gate metrics, golden diagnostics, failure archive, recurrence tracking. | Production retrieval behavior or hidden special cases. |
| Shell | Projection of final answer and bounded detail refs. | Raw tool payloads, workflow authority, provider execution authority. |

## Data Flow

```text
workflow CD
-> web research primitive call
-> batch_query request shaping
-> web_conduit search/fetch/provider admission
-> provider attempts and raw artifact refs
-> batch_query normalization, diagnostics, evidence pack
-> synthesis input binding
-> final response verifier
-> shell projection
```

Optional browser materialization branch:

```text
candidate URL + blocker signal + admitted capability
-> URL safety precheck
-> browser profile compilation
-> browser readiness check
-> materialization adapter
-> final URL safety recheck
-> page readiness/extraction
-> evidence pack row + raw artifact refs
```

## Implementation Target Matrix

| Level 4 ID | Target | Owner Surface | Existing Files | Implementation Rule |
| --- | --- | --- | --- | --- |
| `CLOAK-L4-001` | Tool CD capability contract | Tool CD | `/Users/jay/.openclaw/workspace/core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Declare search, fetch, and browser-materialization capabilities as data. Keep budgets, side effects, safety, and evidence policy in the CD. |
| `CLOAK-L4-002` | Web conduit policy/defaults | Web conduit config and policy reader | `/Users/jay/.openclaw/workspace/core/layer0/ops/config/web_conduit_policy.json`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/041-tooling-inventory-and-policy.rs` | Expose optional capability status without enabling heavy or permission-sensitive providers by default. |
| `CLOAK-L4-003` | URL safety enforcement | Web conduit fetch/materialization boundary | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/025-fetch-utils-and-redirect.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/032-fetch-output-safety.rs` | Validate candidate URLs before fetch/materialization and revalidate redirects/final URLs before any content is promoted. Credentialed URLs are now rejected at fetch preflight, matching the diagnostic/materialization lane. |
| `CLOAK-L4-004` | Browser profile compiler | Provider runtime state plus Tool CD fields | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs` | Compile a deterministic profile projection from admitted fields. Deny caller browser args, raw scripts, CDP commands, local file access, certificate bypass, and debugging flags before any future adapter launch. |
| `CLOAK-L4-005` | Browser materialization adapter | Web conduit provider adapter | Future `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/*browser*materialization*.rs` | Keep adapter default-off and policy-admitted. It may return extraction candidates and artifact refs, not chat-visible text. |
| `CLOAK-L4-006` | Provider readiness lifecycle | Provider runtime state | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/019-fetch-runtime-resolution.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_provider_runtime_parts/021-search-runtime-resolution.rs` | Report missing/disabled/degraded/available states separately from retrieval quality. Do not surprise-install or launch dependencies. |
| `CLOAK-L4-007` | Page readiness and extraction | Fetch readability plus future browser extraction adapter | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/026-fetch-visibility-and-readability.rs` | Convert fetched or rendered page state into bounded main text, metadata, source refs, links, and confidence. Keep raw DOM by ref. |
| `CLOAK-L4-008` | Batch-query candidate/evidence packaging | Batch query primitive | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/016-web-quality-diagnostics.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/019-search-row-candidates.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/020-pipeline.combined.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/021-summary-and-guidance.rs` | Normalize provider results, rank evidence candidates, emit evidence packs, and expose retrieval diagnostics without triggering hidden workflow behavior. |
| `CLOAK-L4-009` | Artifact quarantine/storage | Artifact/media stores and state artifact contracts | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/073-media-store.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/state_artifact_contract_kernel.rs` | Store raw payloads and rendered artifacts by bounded refs with cleanup lifecycle. Chat receives only synthesized output and safe refs. |
| `CLOAK-L4-010` | Workflow CD integration | Research workflow CD and registry | `/Users/jay/.openclaw/workspace/orchestration/src/control_plane/workflows/official/research_synthesize_verify.workflow.json`, `/Users/jay/.openclaw/workspace/orchestration/src/control_plane/workflows/workflow_registry.json` | The workflow may call the web research primitive and require final synthesis, but must not contain browser implementation details. |
| `CLOAK-L4-011` | Assurance/evals | Ops tests, web retrieval gate diagnostics, golden evals | `/Users/jay/.openclaw/workspace/core/layer0/ops/src/batch_query_primitive_parts/043-web-quality-diagnostics-tests.rs`, `/Users/jay/.openclaw/workspace/core/layer0/ops/src/web_conduit_parts/080-tests.rs`, `/Users/jay/.openclaw/workspace/orchestration/src/eval_research_golden.rs`, `/Users/jay/.openclaw/workspace/orchestration/src/eval_research_golden_scoring.rs`, `/Users/jay/.openclaw/workspace/orchestration/src/eval_web_retrieval_gate_diagnostics.rs` | Prove each gate with fixtures first. Track hard retrieval failures separately from soft synthesis quality. |
| `CLOAK-L4-012` | Deferred service/proxy/session/interaction capabilities | Future admitted provider capabilities | Tool CD extension points and future Gateway secret/session surfaces | Keep proxy, session reuse, humanized actions, and browser pools explicit, budgeted, redacted, and disabled unless admitted. |

## Level 3 Mechanic Ownership

| Level 3 ID | Mechanic | Level 4 Owner | Test Surface |
| --- | --- | --- | --- |
| `CLOAK-L3-001` | Provider result normalization | `CLOAK-L4-008`, with provider runtime inputs from `CLOAK-L4-006` | Batch-query diagnostics tests. |
| `CLOAK-L3-002` | Blocker signal extraction | `CLOAK-L4-008`, supported by web-conduit status/errors | Batch-query diagnostics tests plus web-conduit provider fixtures. |
| `CLOAK-L3-003` | Retrieval decision lattice | `CLOAK-L4-008` | Batch-query diagnostics tests and web retrieval gate diagnostics. |
| `CLOAK-L3-004` | Query refinement signal generation | `CLOAK-L4-008` | Batch-query diagnostics tests and golden failure archive. |
| `CLOAK-L3-005` | URL safety mechanics | `CLOAK-L4-003` | Web-conduit SSRF/redirect/output-safety tests plus batch-query projection tests. |
| `CLOAK-L3-006` | Profile compilation mechanics | `CLOAK-L4-004` | Mock-fast profile compiler tests before live browser tests. |
| `CLOAK-L3-007` | Page settle/readiness mechanics | `CLOAK-L4-007` | Fixture tests for shell-only, thin, blocked, and content-ready pages. |
| `CLOAK-L3-008` | Main content extraction mechanics | `CLOAK-L4-007`, artifact refs from `CLOAK-L4-009` | Extraction fixture tests and evidence-pack projection tests. |
| `CLOAK-L3-009` | Evidence promotion scoring | `CLOAK-L4-008` | Evidence-pack scoring tests and golden quality diagnostics. |
| `CLOAK-L3-010` | Retry budget and stop conditions | `CLOAK-L4-008`, workflow budgets in `CLOAK-L4-010` | Batch-query stop-condition tests plus workflow gate diagnostics. |
| `CLOAK-L3-011` | Readiness lifecycle mechanics | `CLOAK-L4-006` | Provider runtime status tests. |
| `CLOAK-L3-012` | Artifact quarantine mechanics | `CLOAK-L4-009` | Artifact quarantine tests and chat-visibility guards. |
| `CLOAK-L3-013` | Mock-fast mechanics tests | `CLOAK-L4-011` | Fixture tests before live integration tests. |

## Phase Order

1. Structural CD and metadata link audit.
   - Confirm web research menu entries point to Tool CD refs.
   - Confirm workflow CD calls the primitive by capability/tool ID, not by provider implementation detail.
   - Confirm diagnostic versions invalidate stale cached payloads.

2. URL safety parity.
   - Move candidate URL safety from diagnostic hints to the web-conduit enforcement boundary where missing.
   - Add mock tests for scheme, credential, localhost/private IP, and redirect revalidation.

3. Browser profile compiler stub and validator.
   - Add a pure compiler/validator that accepts the Tool CD profile shape and returns an admitted profile artifact or structured rejection.
   - Keep runtime browser launch absent or default-off.

4. Provider readiness projection.
   - Expose browser materialization readiness as unavailable/disabled/available/degraded without launching or installing.
   - Keep readiness separate from search result quality.

5. Browser materialization adapter skeleton.
   - Add adapter boundary and response schema with deterministic fake provider first.
   - Keep live Playwright/Puppeteer or service integration behind capability admission.

6. Page readiness and extraction integration.
   - Convert fake/rendered page fixture output into extraction candidates, artifact refs, quality flags, and evidence pack rows.

7. Live provider probes and golden gate updates.
   - Add live probes only after mock-fast contract tests pass.
   - Track browser-materialization gates separately from search/fetch gates so better recovery does not hide weaker search.

## Required Test Classes

| Test Class | Purpose | Scope |
| --- | --- | --- |
| Tool CD schema tests | Prove capability declarations stay parseable and contain required safety/evidence fields. | JSON/CD fixtures. |
| URL safety tests | Prove unsafe targets never reach fetch/browser materialization. | Web conduit unit tests. |
| Profile compiler tests | Prove denied browser controls are rejected before adapter launch. | Pure mock-fast tests. |
| Readiness lifecycle tests | Prove provider readiness does not masquerade as retrieval quality. | Provider runtime state tests. |
| Extraction fixture tests | Prove shell pages, blocker pages, and substantive pages are classified differently. | Fetch/browser extraction fixtures. |
| Evidence pack tests | Prove extracted content becomes bounded evidence with refs, not raw payload chat. | Batch-query diagnostics tests. |
| Workflow CD tests | Prove research CD can call the primitive and return one artifact to the composite/default workflow. | Orchestration workflow tests. |
| Golden diagnostics | Prove hard retrieval failures, soft low-evidence answers, and synthesis failures are archived separately. | Research evals. |

## Exit Criteria

Level 4 is complete when:

- Every Level 3 mechanic has a named owner module or CD.
- Every owner has a first test surface or a documented future module.
- Browser materialization can be added without editing research workflow Rust.
- Workflow CDs can compose the research primitive as a capability call that returns an artifact.
- Raw provider/browser payloads are stored by ref and never projected directly to chat.
- Provider readiness, retrieval quality, evidence quality, and synthesis quality remain separately measurable.

## Not Level 4

The following belong to Level 5 or later:

- Exact Playwright/Puppeteer launch syntax.
- Exact browser profile field names beyond CD-level contract names.
- Proxy rotation implementation.
- Persistent session implementation.
- Humanized mouse/keyboard implementation.
- CDP service pool implementation.
- Live anti-bot site probes as default tests.
- Final answer format rules.

## Current Assessment

The right Level 4 move is to make browser-materialization ownership boring and explicit before adding live browser code. The high-ROI implementation sequence is URL safety parity, profile compiler validation, readiness projection, fake materialization, then extraction/evidence integration. That keeps the capability primitive and prevents it from becoming a special-case research workflow.

## Assimilation Progress

### Slice 1: CD And URL-Safety Parity

Status: integrated and focused-tested.

Implemented:

- Aligned browser-materialization URL safety status values across Tool CD, checked-in web-conduit policy, and default policy.
- Added fetch-boundary credential detection so URLs like `https://user:secret@example.com/...` are rejected before provider execution.
- Added explicit `url_safety_status` fields to fetch SSRF guard output for allowed, invalid, private-network-blocked, and credential-blocked states.
- Added focused tests for credential blocking and guard safety-status projection.

Validation:

- `jq empty core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `git diff --check -- core/layer0/ops/src/web_conduit_parts/031-fetch-transport-and-ssrf.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/020-fetch-policy-and-provider-contract-tests.rs core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/config/web_conduit_policy.json core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib fetch_credentials_in_url_are_blocked_before_execution -- --nocapture`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib ssrf_guard_reports_redirect_target_safety_status -- --nocapture`

Boundary:

This slice does not add browser execution, proxy handling, hidden retry generation, or workflow routing changes.

### Slice 2: Browser Profile Policy Projection

Status: integrated and focused-tested.

Implemented:

- Added a `browser_profile_compilation_v1` projection to browser-materialization runtime metadata.
- Compiled the effective default profile, state scope, denied caller fields, denied launch args, and telemetry fields from the checked-in policy/CD shape.
- Kept the projection default-off with explicit statuses for disabled, adapter-not-ready, and adapter-ready states.
- Preserved the capability boundary that proxy, persistent session, caller launch args, raw CDP, and user scripts require separate admission.
- Added status-surface assertions so the profile envelope remains visible to tooling without launching a browser.
- Surfaced the optional browser lane's profile compilation status and readiness lifecycle on the effective inventory row.

Validation:

- `git diff --check -- core/layer0/ops/src/web_conduit_provider_runtime_parts/018-runtime-web-tools-state_parts/060-runtime-web-family-metadata.rs core/layer0/ops/src/web_conduit_parts/041-tooling-inventory-and-policy.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/010-status-and-provider-catalog-tests.rs docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md docs/workspace/CLOAKBROWSER_WEB_TOOLING_ASSIMILATION_LEDGER.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib status_bootstraps_default_policy_and_receipts_surface -- --nocapture`

Boundary:

This slice does not add live browser execution, profile persistence, proxy handling, hidden retry generation, or browser install behavior.

### Slice 3: Default-Off Browser Materialization Boundary

Status: integrated and focused-tested.

Implemented:

- Added the web-conduit API seam for `browser_materialize_page`.
- Added a CLI command that routes to the same API seam instead of creating a separate execution path.
- Bound the seam to the existing Tool CD/policy contract: URL, `admission_ref`, denied caller controls, profile compilation, readiness lifecycle, and raw-payload chat boundary.
- Reused fetch URL-safety/SSRF checks before any adapter state is considered.
- Kept disabled, adapter-not-ready, and stub-only outcomes fail-closed with `browser_launch_attempted=false`.
- Added mock-fast contract tests for default-off state, caller control rejection, credentialed URL rejection, and enabled-without-adapter readiness.

Validation:

- `git diff --check -- core/layer0/ops/src/web_conduit.rs core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs core/layer0/ops/src/web_conduit_parts/034-browser-materialization.rs core/layer0/ops/src/web_conduit_parts/070-cli-run.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests.rs core/layer0/ops/src/web_conduit_parts/080-tests_parts/010-mod-tests_parts/050-browser-materialization-contract-tests.rs docs/workspace/CLOAKBROWSER_LEVEL4_IMPLEMENTATION_STRUCTURE_MAP.md`
- `env TMPDIR=/Users/jay/.openclaw/workspace/target/tmp CARGO_INCREMENTAL=0 cargo test -q -p infring-ops-core --lib browser_materialization -- --nocapture`

Boundary:

This slice is still not a live browser adapter. It is the primitive call boundary that a later adapter must satisfy.
