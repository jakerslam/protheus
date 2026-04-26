# Orchestration Control-Plane Parity Map

Status: active transition map
Owner: surface/orchestration
Purpose: keep the Infring control plane understandable against OpenHands and OpenFang while preventing coordination authority from scattering into Shell/Core.

## Boundary Rule

Core decides truth and permission.
Orchestration decides what should happen next.
Shell shows and collects interaction.

## OpenHands Parity

OpenHands separates the server/conversation manager, runtime manager, action execution server, event stream, and memory condenser.

Infring mapping:

- OpenHands EventStream -> Infring workflow/state streams under `response_workflow`, `local/state/ops/orchestration/**`, and eval trace artifacts.
- OpenHands action execution -> Infring typed tool-family contracts (`workspace`, `web`, `memory`, `agent`, `shell`, `browser`) with request, observation, and receipt binding.
- OpenHands runtime/sandbox -> Infring Core/Gateway execution authority, with Orchestration only recommending and sequencing tool requests.
- OpenHands memory condenser -> Infring memory/context surfaces, with Orchestration consuming snapshots and packaging result context.
- OpenHands UI event stream -> Infring UI harness streams (`workflow_state`, `agent_internal_notes`, `tool_trace`, `eval_trace`, `final_answer`).

Parity requirement:

- The control plane must be an event-sourced action/observation loop, not a chat-text workflow script.
- Every user turn should have a typed chain: user input, workflow decision, action request, observation, and final LLM response.

Assimilated OpenHands controller mechanics:

- `openhands/controller/__init__.py` -> keep the public controller surface narrow; expose the controller boundary, not state/runtime internals.
- `openhands/controller/agent.py` -> require unique agent registration, initialized prompt context before system-message creation, agent-sourced system messages, and MCP tool-name dedupe.
- `openhands/controller/agent_controller.py` -> model orchestration as event-sourced step eligibility with delegate forwarding, hidden-event filtering, typed exception status mapping, recall routing, and pending-action reset evidence.
- `openhands/controller/replay.py` -> replay only user/agent actions, drop environment/null observations, suppress mid-replay wait-for-response, and clear event IDs before replay insertion.
- `openhands/controller/state/control_flags.py` -> project iteration/budget limit semantics into orchestration run budgets while keeping canonical quota enforcement in Kernel.
- `openhands/controller/state/state.py` -> persist resumable state without history/cache fields, restore through LOADING, and convert legacy iteration state into explicit control flags.
- `openhands/controller/state/state_tracker.py` -> rebuild filtered event history, compact delegate ranges, emit trajectories for eval/replay, snapshot metrics, and persist state plus conversation metrics.
- `openhands/controller/stuck.py` -> detect repeated action/observation loops, repeated error loops, monologues, action/observation patterns, and context-window loops with recovery-ready loop metadata.
- `openhands/runtime/action_execution_server.py` -> keep sandbox action execution behind API-key verification, async mutation locks, lazy browser readiness, memory monitoring, editor error envelopes, and serialized observations.
- `openhands/runtime/base.py` -> keep runtime provider boundaries abstract while projecting event-stream subscription, plugin loading, provider-token env hydration, status callbacks, and command retry semantics.
- `openhands/runtime/README.md` -> keep runtime architecture explicit: runtime interface, action-execution server, event-stream action handling, observation generation, plugin setup, and isolation warnings.
- `openhands/runtime/__init__.py` -> project runtime class resolution, default/runtime alias mapping, optional third-party runtime discovery, custom runtime fallback, and supported-runtime diagnostics.
- `openhands/runtime/browser/__init__.py` -> keep browser public surface narrow (`browse` only), with browser process internals staying behind the runtime boundary.
- `openhands/runtime/browser/base64.py` -> preserve browser screenshots and set-of-marks as replayable PNG/base64 observation payloads.
- `openhands/runtime/browser/browser_env.py` -> project browser process isolation, eval-mode goal/reward commands, DOM text extraction, serializable observations, timeout handling, and forced shutdown semantics.
- `openhands/runtime/browser/utils.py` -> project browse URL vs interactive action routing, accessibility-tree text blocks, screenshot persistence, return-axtree elision, and browser error observation shaping.
- `openhands/runtime/builder/__init__.py` -> keep runtime-builder public exports narrow and provider internals behind Gateway/runtime build boundaries.
- `openhands/runtime/builder/base.py` -> define build/image-existence provider interface semantics and effective image-name reporting.
- `openhands/runtime/builder/docker.py` -> project BuildKit/Podman compatibility, buildx probing/bootstrap, cache flags, rolling build logs, retagging, verification, and local/remote image existence checks.
- `openhands/runtime/builder/remote.py` -> project remote build-context packaging, rate-limit retry, build-status polling, terminal failure mapping, and image-existence endpoint checks.
- `openhands/runtime/file_viewer_server.py` -> project localhost-only file viewing, absolute-path checks, missing-file/directory rejection, generated viewer HTML, and legacy no-auth warning semantics as runtime observation contracts.
- `openhands/runtime/impl/__init__.py` -> keep provider implementation exports explicit while preventing orchestration or shell from treating concrete provider modules as canonical runtime authority.
- `openhands/runtime/impl/action_execution/action_execution_client.py` -> project serialized action execution over HTTP, single-action concurrency, liveness probes, file transfer endpoints, timeout defaults, confirmation handling, and observation deserialization.
- `openhands/runtime/impl/cli/__init__.py` -> keep the CLI runtime public surface narrow and implementation-scoped.
- `openhands/runtime/impl/cli/cli_runtime.py` -> project no-sandbox local execution warnings, workspace setup, Windows PowerShell support, secret-safe env hydration, subprocess streaming, and termination fallback semantics.
- `openhands/runtime/impl/docker/containers.py` -> project prefix-scoped runtime container cleanup, defensive Docker API error handling, and Docker client lifecycle closure.
- `openhands/runtime/impl/docker/docker_runtime.py` -> project Docker runtime lifecycle, port locks, image build fallback, attach-vs-create behavior, readiness gating, optional log streaming, additional networks, and volume/overlay handling.
- `openhands/runtime/impl/kubernetes/README.md` -> project Kubernetes runtime operator prerequisites, required runtime config, KIND/mirrord development topology, ingress/RBAC/PVC/resource configuration, and troubleshooting flows.
- `openhands/runtime/impl/kubernetes/kubernetes_runtime.py` -> project pod/service naming, namespace binding, config fail-closed checks, attach-vs-create behavior, readiness polling, cleanup, scheduling constraints, and status transitions.
- `openhands/runtime/impl/local/__init__.py` -> keep the local runtime public surface narrow and implementation-scoped.
- `openhands/runtime/impl/local/local_runtime.py` -> project direct host action-execution server lifecycle, no-sandbox warnings, dependency checks, workspace selection, warm-server reuse, session API keys, process/log tracking, and readiness gating.
- `openhands/runtime/impl/remote/remote_runtime.py` -> project API-key-gated remote runtime lifecycle, attach/start/resume behavior, remote image build/existence checks, session API-key propagation, readiness polling, and VSCode URL shaping.
- `openhands/runtime/mcp/config.json` -> project explicit empty-default MCP server/tool bootstrap semantics.
- `openhands/runtime/mcp/proxy/README.md` -> project FastMCP proxy lifecycle, in-memory tool configuration, FastAPI mounting, update/remount, auth shape, and shutdown semantics.
- `openhands/runtime/mcp/proxy/__init__.py` -> keep the MCP proxy public surface narrow around the proxy manager.
- `openhands/runtime/mcp/proxy/manager.py` -> project FastMCP proxy construction, optional static-token auth, empty-config skip behavior, SSE FastAPI mounting, double-response-start protection, stdio-server remapping, and update/remount lifecycle.
- `openhands/runtime/plugins/__init__.py` -> project explicit runtime plugin exports and name-to-plugin registry mapping for `jupyter`, `agent_skills`, and `vscode`.
- `openhands/runtime/plugins/agent_skills/README.md` -> project sandbox-only agent skill policy, inclusion criteria, IPython usage, retrieval/help flow, and anti-wrapper-bloat guidance.
- `openhands/runtime/plugins/agent_skills/__init__.py` -> project agent-skills requirement documentation hydration, no-op initialization, and explicit unsupported direct run behavior.
- `openhands/runtime/plugins/agent_skills/agentskills.py` -> project dynamic skill exports, optional repo ops, generated callable documentation, and file-editor function export.
- `openhands/runtime/plugins/agent_skills/file_editor/README.md` -> project external editor lineage, MIT-license attribution, and runtime-skill boundary constraints for imported edit patterns.
- `openhands/runtime/plugins/agent_skills/file_editor/__init__.py` -> keep file-editor public export narrow around the openhands-aci singleton callable.
- `openhands/runtime/plugins/agent_skills/file_ops/__init__.py` -> project explicit file-operation skill export hydration through declared `__all__`.
- `openhands/runtime/plugins/agent_skills/file_ops/file_ops.py` -> project bounded file-window navigation, current-file state, line clamping, search/find ergonomics, hidden-file elision, broad-match narrowing, and visible recovery errors.
- `openhands/runtime/plugins/agent_skills/file_reader/__init__.py` -> project explicit file-reader skill export hydration through declared `__all__`.
- `openhands/runtime/plugins/agent_skills/file_reader/file_readers.py` -> project PDF/DOCX/LaTeX/PPTX parsing, credential-gated audio/image/video tools, multimodal frame sampling bounds, and visible parser error output.
- `openhands/runtime/plugins/agent_skills/repo_ops/__init__.py` -> project explicit repository-operation skill export hydration through declared `__all__`.
- `openhands/runtime/plugins/agent_skills/repo_ops/repo_ops.py` -> project code entity lookup, code snippet search, and repository tree exploration through runtime semantic-navigation skills.
- `openhands/runtime/plugins/agent_skills/utils/config.py` -> project late-bound sandbox credential/model/token config for OpenAI-backed runtime skills.
- `openhands/runtime/plugins/agent_skills/utils/dependency.py` -> project explicit declared-function import and fail-loud missing export behavior for runtime skill registries.
- `openhands/runtime/plugins/jupyter/__init__.py` -> project Jupyter kernel-gateway startup, local-vs-sandbox command shaping, Windows subprocess fallback, user switching, interpreter discovery, lazy kernel client initialization, and IPython-only action support.
- `openhands/runtime/plugins/jupyter/execute_server.py` -> project kernel creation retries, websocket heartbeat/reconnect, message-id correlation, structured text/image output collection, ANSI stripping, timeout interruption, and Tornado execute endpoint behavior.
- `openhands/runtime/plugins/requirement.py` -> project the minimal runtime plugin interface and named plugin requirement declarations.
- `openhands/runtime/plugins/vscode/__init__.py` -> project VSCode plugin platform/user checks, workspace settings installation, port/token setup, OpenVSCode launch, path routing support, readiness wait, and unsupported direct-run behavior.
- `openhands/runtime/plugins/vscode/settings.json` -> project minimal default operator IDE settings without turning them into runtime authority.
- `openhands/runtime/runtime_status.py` -> project explicit runtime lifecycle, setup, error, LLM, rate-limit, git-auth, retry, and memory status vocabulary.
- `openhands/runtime/utils/__init__.py` -> keep runtime utility public exports narrow around number-matrix display and TCP-port discovery.
- `openhands/runtime/utils/bash.py` -> project bashlex command splitting, special-character escaping, tmux shell session setup, PS1 metadata, cwd tracking, output trimming, completed/no-change handling, and recovery guidance.
- `openhands/runtime/utils/bash_constants.py` -> project reusable timeout guidance for waiting, interacting, interrupting, or setting future timeout parameters.
- `openhands/runtime/utils/command.py` -> project action-execution server startup command construction, plugin args, browsergym args, username/user-id resolution, workspace binding, and browser-disable flag.
- `openhands/runtime/utils/edit.py` -> project LLM-assisted draft editing, strict updated-code extraction, edit-range validation, append semantics, max edit-size guardrails, relevant-snippet recovery hints, diff generation, and metric propagation.
- `openhands/runtime/utils/file_viewer.py` -> project supported-extension checks, local file existence checks, MIME detection, base64 PDF/image embedding, PDF.js rendering, and read-only viewer error display.
- `openhands/runtime/utils/files.py` -> project sandbox-to-host workspace path resolution, workspace escape denial, bounded line reads, partial write insertion, parent directory creation, and typed file observations.
- `openhands/runtime/utils/git_changes.py` -> project workspace git change summaries with ref fallback, rename/copy normalization, untracked inclusion, nested repo handling, sorted output, and JSON error fallback.
- `openhands/runtime/utils/git_diff.py` -> project single-file git diff inspection with closest-repo discovery, size guardrails, safe git-show quoting, original/modified capture, and missing-file handling.
- `openhands/runtime/utils/git_handler.py` -> project shell-backed git branch/change/diff operations, runtime script fallback installation, JSON parsing, safe path quoting, and command-result envelopes.
- `openhands/runtime/utils/log_capture.py` -> project scoped async log capture with handler replacement, level override, in-memory stream collection, and guaranteed logger restoration.
- `openhands/runtime/utils/log_streamer.py` -> project Docker container log streaming thread lifecycle, decoded line forwarding, stop-event shutdown, generator closure, and stream error reporting.
- `openhands/runtime/utils/memory_monitor.py` -> project opt-in runtime memory monitoring, logger stream redirection, daemon monitoring thread, child-process inclusion, PSS backend, and visible monitor failure logs.
- `openhands/runtime/utils/port_lock.py` -> project file-based port locks, fcntl/atomic-file fallback, lock-before-bind port discovery, release cleanup, context-manager support, and stale lock cleanup.
- `openhands/runtime/utils/request.py` -> project HTTP 429 retry, response detail extraction, failure response cleanup, and enriched request errors.
- `openhands/runtime/utils/runtime_build.py` -> project runtime image repo/tag derivation, scratch/versioned/lock build strategy, Dockerfile rendering, build-folder prep, source/lock hashing, image reuse, and dry-run generation.
- `openhands/runtime/utils/runtime_init.py` -> project runtime user creation/reconciliation, working-directory creation, sandbox group ownership, and group-write permissions.
- `openhands/runtime/utils/runtime_templates/Dockerfile.j2` -> project runtime image template dependency setup, Node/Python/uv/micromamba/Poetry bootstrap, Docker support, OpenVSCode setup, extensions, optional Playwright, and workspace permissions.
- `openhands/runtime/utils/singleton.py` -> record empty legacy singleton marker status with no public runtime exports.
- `openhands/runtime/utils/system.py` -> project TCP port probing, randomized bounded port discovery, collision backoff, and ASCII numeric matrix rendering.
- `openhands/runtime/utils/tenacity_stop.py` -> project shutdown-listener-aware retry stop behavior for long-running retry loops.
- `openhands/runtime/utils/vscode-extensions/hello-world/extension.js` -> project VSCode command registration, subscription cleanup ownership, and operator notification smoke-check behavior.
- `openhands/runtime/utils/vscode-extensions/hello-world/package.json` -> project VSCode extension metadata, engine compatibility, command activation, command contribution, and entrypoint declaration.
- `openhands/runtime/utils/vscode-extensions/memory-monitor/README.md` -> project operator-facing memory observability goals, status-bar interaction, commands, detailed views, cross-platform intent, and licensing.
- `openhands/runtime/utils/vscode-extensions/memory-monitor/extension.js` -> project memory-monitor activation, command registration, subscription ownership, context attachment, and default auto-start.
- `openhands/runtime/utils/vscode-extensions/memory-monitor/memory_monitor.js` -> project status-bar telemetry, bounded memory history, detailed webview rendering, process table integration, Chart.js history visualization, and request-update loop.
- `openhands/runtime/utils/vscode-extensions/memory-monitor/package.json` -> project memory-monitor extension metadata, startup activation, commands, command-palette grouping, engine compatibility, and monitoring category.
- `openhands/runtime/utils/vscode-extensions/memory-monitor/process_monitor.js` -> project Linux/macOS/Windows process memory collection, ps/WMIC parsing, top-process sorting, and unsupported-platform errors.
- `openhands/runtime/utils/windows_bash.py` -> project pythonnet/CoreCLR and PowerShell SDK discovery, persistent runspace setup, CWD confirmation, job output/error receipt, active-job stop semantics, and .NET diagnostics.
- `openhands/runtime/utils/windows_exceptions.py` -> project clean .NET/CoreCLR/PowerShell SDK missing diagnostics with optional detail payloads.
- `openhands/app_server/README.md` -> project FastAPI V1 integration, AgentSDK bridge positioning, API module taxonomy, lifecycle surfaces, event/callback/git/sandbox endpoints, secrets/settings/status/user APIs, and web-client routing.
- `openhands/app_server/__init__.py` -> record empty app-server package marker with no hidden public imports.
- `openhands/app_server/app_conversation/README.md` -> project sandboxed conversation lifecycle management, CRUD service abstraction, live status tracking, router ownership, search/filtering, pagination, and sandbox integration.
- `openhands/app_server/app_conversation/__init__.py` -> record lightweight app-conversation namespace marker with explicit submodule imports required.
- `openhands/app_server/app_conversation/app_conversation_info_service.py` -> project conversation metadata search/count/get/batch/delete/save, sub-conversation lookup, sandbox reference counting, stats-event processing, and injector boundaries.
- `openhands/app_server/app_conversation/app_conversation_models.py` -> project conversation schema contracts for agent type, plugin specs, metadata, start/update requests, start-task statuses, pages, skills, and hooks.
- `openhands/app_server/app_conversation/app_conversation_router.py` -> project protected FastAPI conversation router, dependency wiring, agent-server context derivation, sandbox readiness checks, exposed URL lookup, and bounded search endpoint semantics.
- `openhands/app_server/app_conversation/app_conversation_service.py` -> project abstract sandboxed conversation lifecycle service, streamed start-task progression, setup execution, update/delete semantics, and conversation export contract.
- `openhands/app_server/app_conversation/app_conversation_service_base.py` -> project shared conversation service base for project-root derivation, skill loading and merge, agent context updates, repository preparation, setup scripts, git hooks, and skill setup progression.
- `openhands/app_server/app_conversation/app_conversation_start_task_service.py` -> project start-task search/count/get/batch/save/delete operations, pagination/sort filters, and injector-selected implementation boundary.
- `openhands/app_server/app_conversation/git/README.md` -> record conversation git support directory as setup-scoped configuration assets.
- `openhands/app_server/app_conversation/git/pre-commit.sh` -> project installed pre-commit delegation to `.openhands/pre-commit.sh`, exit-code propagation, warning-on-missing, and non-blocking absent-hook behavior.
- `openhands/app_server/app_conversation/hook_loader.py` -> project hook project-dir resolution, agent-server `/api/hooks` retrieval, session API key headers, empty-hook normalization, and startup-safe graceful failure.
- `openhands/app_server/app_conversation/live_status_app_conversation_service.py` -> project persisted metadata plus live sandbox status composition, start-task streaming persistence, parent inheritance, suggested task application, sandbox waiting, grouped workdirs, setup orchestration, and planning-agent no-execute instruction.
- `openhands/app_server/app_conversation/skill_loader.py` -> project app-server-as-thin-proxy skill loading, provider-aware org config, sandbox exposed URL config, agent-server `/api/skills` retrieval, session API key headers, and graceful empty-list failure.
- `openhands/app_server/app_conversation/sql_app_conversation_info_service.py` -> project SQL conversation metadata persistence, V1 search/count/filter/sort/page, parent/sub-conversation lookup, sandbox reference counts, metrics/token storage, and permission-wrapper boundary.
- `openhands/app_server/app_conversation/sql_app_conversation_start_task_service.py` -> project SQL start-task persistence, user-scoped search/count/get/batch/save/delete, missing-preserving ordered batch results, and injector-scoped DB/user context.
- `openhands/app_server/app_lifespan/alembic.ini` -> project Alembic script-location, sys.path, path separator, dynamic DB URL ownership, optional format/lint hooks, and logging config.
- `openhands/app_server/app_lifespan/alembic/README` -> project startup SQLite migration purpose, DbSessionInjector connectivity, declarative-base model discovery, and autogenerate command docs.
- `openhands/app_server/app_lifespan/alembic/env.py` -> project Alembic env bootstrap, log suppression, model metadata registration, dynamic Postgres/SQLite URL selection, offline migrations, and online DbSession engine reuse.
- `openhands/app_server/app_lifespan/alembic/script.py.mako` -> project typed Alembic migration template with metadata header, standard imports, revision identifiers, upgrade/downgrade functions, and empty-body pass fallback.
- `openhands/app_server/app_lifespan/alembic/versions/001.py` -> project initial app-server schema for start tasks, event callbacks/results, remote sandboxes, conversation metadata, indexes, and reverse-order downgrade cleanup.
- `openhands/app_server/app_lifespan/alembic/versions/002.py` -> project event-callback status lifecycle, updated-at timestamp, event-id string migration, index rebuild, and reversible downgrade.
- `openhands/app_server/app_lifespan/alembic/versions/003.py` -> project parent conversation metadata column and indexed sub-conversation lookup migration.
- `openhands/app_server/app_lifespan/alembic/versions/004.py` -> project public conversation metadata flag, public visibility index, and reversible downgrade.
- `openhands/app_server/app_lifespan/alembic/versions/005.py` -> project StoredConversationMetadata schema convergence by dropping legacy `github_user_id` and relaxing `user_id` nullability through batch alteration.
- `openhands/app_server/app_lifespan/alembic/versions/006.py` -> project remote-sandbox `session_api_key_hash` persistence, indexed lookup, and reversible batch alteration.
- `openhands/app_server/app_lifespan/alembic/versions/007.py` -> project server-side pending message queue table for messages submitted before conversation readiness.
- `openhands/app_server/app_lifespan/alembic/versions/008.py` -> project nullable JSON conversation tags for automation context and skills-used metadata.
- `openhands/app_server/app_lifespan/app_lifespan_service.py` -> project discriminated async FastAPI lifespan boundary with explicit open/close hooks.
- `openhands/app_server/app_lifespan/oss_app_lifespan_service.py` -> project optional startup Alembic upgrade, absolute migration config, controlled working-directory switch, and CWD restoration.
- `openhands/app_server/config.py` -> project environment-derived defaults, provider URL resolution, storage/sandbox injector selection, conversation/pending-message/user/JWT/httpx/DB DI wiring, and global config/dependency helpers.
- `openhands/app_server/config_api/config_models.py` -> project LLM model/provider response schemas with verified flags and pagination envelopes.
- `openhands/app_server/config_api/config_router.py` -> project protected model/provider search endpoints with query/filter/pagination, verified model enrichment, unique provider extraction, and verified-first sorting.
- `openhands/app_server/errors.py` -> project HTTPException-based error taxonomy for general, authentication, permission, and sandbox failures.
- `openhands/app_server/event/README.md` -> project event management docs covering storage, retrieval, filtering, sorting, pagination, streaming, and multiple backends.
- `openhands/app_server/event/aws_event_service.py` -> project S3-backed event persistence with role-based AWS auth, JSON serialization, missing-key normalization, prefix search, and injector-scoped user/conversation context.
- `openhands/app_server/event/event_router.py` -> project protected conversation event search/count/batch-get endpoints with kind/time filters, bounded batch size, and UUID conversion.
- `openhands/app_server/event/event_service.py` -> project abstract event service for get/search/count/internal-save/batch-get plus discriminated injector boundary.
- `openhands/app_server/event/event_service_base.py` -> project shared event storage path derivation, user/conversation owner scoping, executor-isolated backend IO, event filtering/sorting/pagination/counting, save, and batch-get behavior.
- `openhands/app_server/event/event_store.py` -> record empty event-store module marker with no runtime event storage authority in the current vendor snapshot.
- `openhands/app_server/event/filesystem_event_service.py` -> project local filesystem JSON event persistence, parent-directory creation, glob path search, read-error normalization, and injector-scoped persistence/user/conversation context.
- `openhands/app_server/event/google_cloud_event_service.py` -> project GCS-backed event persistence, missing-blob normalization, JSON serialization, prefix listing, and injector-scoped bucket/user/conversation context.
- `openhands/app_server/event_callback/README.md` -> project webhook/event-callback docs for registration, event filtering, callback result tracking, retry logic, secure auth, and router/service/storage ownership.
- `openhands/app_server/event_callback/__init__.py` -> project explicit event-callback processor registration/export boundary while avoiding circular title-processor imports.
- `openhands/app_server/event_callback/event_callback_models.py` -> project callback statuses, dynamic event-kind typing, processor interface, redacted logging processor, create request, persisted callback state, and pagination.
- `openhands/app_server/event_callback/event_callback_result_models.py` -> project callback result statuses, result sort vocabulary, result payload schema, and pagination.
- `openhands/app_server/event_callback/event_callback_service.py` -> project abstract callback CRUD/search/save/execute service with missing-preserving batch get and injector boundary.
- `openhands/app_server/event_callback/set_title_callback_processor.py` -> project message-event-only title polling processor with admin service context, Docker URL rewrite, retry-on-unavailable title, metadata update, and callback disable-on-success.

## OpenFang Parity

OpenFang separates Kernel, Runtime, Memory, Hands, API, Channels, Extensions, and Migration. Its Kernel owns workflows, RBAC, metering, scheduler, background execution, and capability gates.

Infring mapping:

- OpenFang workflow engine -> `surface/orchestration/src/control_plane/**` workflow selection, typed graph compilation, sequencing, and recovery.
- OpenFang capability manifests -> future Infring orchestration-managed agent capability manifests, while Core retains enforcement.
- OpenFang agent loop -> Infring final LLM stage plus tool observation loop.
- OpenFang scheduler/metering -> Core truth; Orchestration may propose budgets and termination limits but does not enforce canonical quotas.
- OpenFang Hands -> future Infring reusable orchestration templates with manifest-bound tools and lifecycle state.
- OpenFang audit trail -> Core receipts plus Orchestration correlation metadata.

Parity requirement:

- Workflow runs must have terminal states, run budgets, loop guards, retry/escalation policy, and telemetry streams.
- Tool access must be typed by family and bound to receipts.

## Infring Control-Plane Responsibilities

Canonical Rust home: `surface/orchestration/src/**`

Current required control-plane concerns:

- Decomposition
- Coordination
- Sequencing
- Recovery
- Result packaging
- Workflow graph validation
- Structured gate contracts
- Telemetry stream separation
- Tool-family request/observation contracts

Forbidden leakage:

- No system-authored fallback text in visible chat.
- No automatic task/info or workflow-route classifier deciding tool use before the LLM gate.
- No control-plane coordination authority in Shell wrappers.
- No canonical policy truth or execution admission in Orchestration.

## Guard

The active guard is:

`cargo run --quiet --manifest-path surface/orchestration/Cargo.toml --bin workflow_contract_guard -- --strict=1`

It checks that workflow JSON compiles into typed graphs, structured gates expose only multiple-choice/text-input shapes, tool families have request/observation/receipt contracts, run budgets and terminal states exist, telemetry streams are separate, and visible chat remains LLM-final-only.

## OpenHands Assimilation Wave: API-02F + API-03A (2026-04-26)

- `openhands/app_server/event_callback/sql_event_callback_service.py` -> `openhands_sql_event_callback_service_contracts`: SQL callback/result persistence, typed JSON processor storage, filtered active callback execution, exception result capture, callback self-mutation persistence, and DB-session injector scoping.
- `openhands/app_server/event_callback/util.py` -> `openhands_event_callback_util_contracts`: fail-closed conversation lookup, running sandbox validation, session API key requirement, agent-server exposed URL discovery, and Docker-safe URL rewriting.
- `openhands/app_server/event_callback/webhook_router.py` -> `openhands_event_webhook_router_contracts`: session-key sandbox auth, sandbox-owner request context, conversation upsert, automation tag trigger detection, event/stat persistence, sequential background callback execution, JWT-scoped secret retrieval, and tool import hydration.
- `openhands/app_server/git/git_models.py` -> `openhands_git_model_contracts`: provider search sort vocabulary plus paginated installation, repository, branch, and suggested-task envelopes.
- `openhands/app_server/git/git_router.py` -> `openhands_git_router_contracts`: provider-token-gated git surfaces, provider-specific installation discovery, repository/branch search and listing pagination, suggested-task pagination, and explicit unsupported-operation failures.

## OpenHands Assimilation Wave: API-03B (2026-04-26)

- `openhands/app_server/pending_messages/__init__.py` -> `openhands_pending_messages_package_contracts`: explicit pending-message domain model, response, abstract service, SQL service, and injector public exports.
- `openhands/app_server/pending_messages/pending_message_models.py` -> `openhands_pending_message_model_contracts`: UUID-backed queued message identity, task/conversation ID flexibility, default user role, typed text/image content, UTC creation timestamps, and queue position responses.
- `openhands/app_server/pending_messages/pending_message_router.py` -> `openhands_pending_message_router_contracts`: authenticated conversation-scoped enqueue endpoint, JSON validation, typed multimodal content parsing, per-conversation max-10 queue limit, service-backed enqueue, and queue position logging.
- `openhands/app_server/pending_messages/pending_message_service.py` -> `openhands_pending_message_service_contracts`: abstract queue contract, SQL persistence, JSON content serialization/deserialization, FIFO retrieval, count/delete/update operations, task-to-conversation ID migration, and DB-session injector scoping.
- `openhands/app_server/sandbox/README.md` -> `openhands_sandbox_docs_contracts`: secure sandbox execution boundary, lifecycle management, Docker implementation, sandbox spec/template service, router ownership, multiple backend support, monitoring, and user-scoped access control.

## OpenHands Assimilation Wave: API-03C (2026-04-26)

- `openhands/app_server/sandbox/docker_sandbox_service.py` -> `openhands_docker_sandbox_service_contracts`: Docker sandbox lifecycle, runtime env defaults, container status normalization, exposed URL/session-key extraction, health checks with startup grace, prefixed lookup, sandbox limit enforcement, host/bridge networking, CORS env indexing, KVM passthrough, Docker run configuration, and cleanup.
- `openhands/app_server/sandbox/docker_sandbox_spec_service.py` -> `openhands_docker_sandbox_spec_service_contracts`: default agent-server image spec, runtime environment construction, global Docker client reuse, missing-image pull-once behavior, concurrent image pulls, Docker API error normalization, and periodic pull progress logging.
- `openhands/app_server/sandbox/preset_sandbox_spec_service.py` -> `openhands_preset_sandbox_spec_service_contracts`: fixed sandbox spec inventory, offset pagination, invalid page fallback, lookup-by-id, and first-spec default selection.
- `openhands/app_server/sandbox/process_sandbox_service.py` -> `openhands_process_sandbox_service_contracts`: local process-backed sandbox lifecycle, dedicated directories, unique port allocation, per-sandbox session keys, process readiness checks, psutil status mapping, health-checked exposed URLs, in-memory process tracking, graceful/forced termination, directory cleanup, and injector scoping.
- `openhands/app_server/sandbox/process_sandbox_spec_service.py` -> `openhands_process_sandbox_spec_service_contracts`: process sandbox default spec, Python module command, VS Code disabled default, inherited agent-server environment, empty working directory, and preset spec service injection.

## OpenHands Assimilation Wave: API-03D (2026-04-26)

- `openhands/app_server/sandbox/remote_sandbox_service.py` -> `openhands_remote_sandbox_service_contracts`: remote runtime API adaptation, local SQL shadow state, SHA-256 session-key hashes and legacy backfill, user-scoped selection, runtime status mapping, batch runtime lookup, webhook/CORS/worker environment initialization, remote lifecycle operations, service URL construction, polling-based conversation/event refresh, and order-preserving batch get.
- `openhands/app_server/sandbox/remote_sandbox_spec_service.py` -> `openhands_remote_sandbox_spec_service_contracts`: remote agent-server image spec, binary command on port 60000, runtime logging/conversation/bash event environment, VS Code port wiring, workspace project working directory, and preset service injection.
- `openhands/app_server/sandbox/sandbox_models.py` -> `openhands_sandbox_model_contracts`: sandbox status vocabulary, exposed URL shape, standard service names, sandbox info/session-key visibility semantics, paginated sandbox responses, and secret metadata responses without raw values.
- `openhands/app_server/sandbox/sandbox_router.py` -> `openhands_sandbox_router_contracts`: authenticated sandbox search/batch/lifecycle routes, max-100 batch guard, session-key-scoped secret list/value endpoints, sandbox-owner auth context, custom-secret/provider-token lookup order, and plain-text secret delivery with 404 fallback.
- `openhands/app_server/sandbox/sandbox_service.py` -> `openhands_sandbox_service_contracts`: abstract sandbox lifecycle interface, session-key lookup, concurrent batch get, wait-for-running with optional agent-server liveness probe, agent-server URL extraction and Docker-safe rewrite, oldest-running cleanup, and discriminated injector boundary.

## OpenHands Assimilation Wave: API-03E (2026-04-26)

- `openhands/app_server/sandbox/sandbox_spec_models.py` -> `openhands_sandbox_spec_model_contracts`: sandbox template identity, optional command, UTC created-at default, initial environment variables, default working directory, and paginated spec response envelopes.
- `openhands/app_server/sandbox/sandbox_spec_router.py` -> `openhands_sandbox_spec_router_contracts`: authenticated sandbox-spec search, bounded batch lookup, query pagination, service delegation, max-100 batch guard, missing-preserving results, and immutable server-wide spec-list assumption.
- `openhands/app_server/sandbox/sandbox_spec_service.py` -> `openhands_sandbox_spec_service_contracts`: read-only spec service interface, fail-closed default spec lookup, concurrent batch lookup, agent-server image environment override, LLM environment auto-forwarding, explicit `OH_AGENT_SERVER_ENV` overrides, and injector boundary.
- `openhands/app_server/sandbox/session_auth.py` -> `openhands_session_auth_contracts`: centralized `X-Session-API-Key` validation, admin-scoped sandbox lookup, running-sandbox-only enforcement, SAAS owner requirement, authenticated caller ownership verification, and explicit 401/403 failure modes.
- `openhands/app_server/secrets/secrets_models.py` -> `openhands_secret_model_contracts`: no-value secret listing model, create-time `SecretStr` value handling, optional descriptions, value exclusion from list responses, and paginated custom-secret response envelopes.

## OpenHands Assimilation Wave: API-03F + API-04A (2026-04-26)

- `openhands/app_server/secrets/secrets_router.py` -> `openhands_secret_router_contracts`: authenticated secrets routes, provider-token validation and storage, provider-token deletion, custom secret search/create/update/delete, no-value listing, name-cursor pagination, duplicate-name guards, and provider-token preservation.
- `openhands/app_server/services/README.md` -> `openhands_services_docs_contracts`: core service ownership for authentication, token management, security operations, JWT signing/verification/encryption, JWE support, multi-key rotation, configurable algorithms, and secure token validation.
- `openhands/app_server/services/db_session_injector.py` -> `openhands_db_session_injector_contracts`: legacy DB env defaults, SQLite/PostgreSQL/GCP engine selection, sync/async engine and session maker caching, request-state session reuse, commit/rollback/close semantics, keep-open toggle, and driver diagnostics.
- `openhands/app_server/services/httpx_client_injector.py` -> `openhands_httpx_client_injector_contracts`: request-state `httpx.AsyncClient` reuse, timeout configuration, handshake minimization, connection-pool cleanup, and keep-open toggle.
- `openhands/app_server/services/injector.py` -> `openhands_injector_base_contracts`: generic async dependency injection interface, Starlette `State` alias, nested injector state reuse, async context-manager wrapper, and FastAPI dependency adapter.

## OpenHands Assimilation Wave: API-04B (2026-04-26)

- `openhands/app_server/services/jwt_service.py` -> `openhands_jwt_service_contracts`: active-key requirement, newest active default key, JWS creation/verification, JWE creation/decryption, protected-header validation, algorithm pinning, SHA-256 symmetric key derivation, invalid-token handling, and cached default-key injector.
- `openhands/app_server/settings/settings_router.py` -> `openhands_settings_router_contracts`: authenticated settings load/store, legacy secrets migration, provider-token presence metadata without values, sensitive key elision, LLM base URL normalization, diff-only writes, legacy nested-key rejection, global runtime/git config updates, schema endpoints, and one-time settings secret-store invalidation.
- `openhands/app_server/status/status_router.py` -> `openhands_status_router_contracts`: liveness, health, readiness, and server-info endpoints with explicit readiness/liveness separation for future invariants.
- `openhands/app_server/status/system_stats.py` -> `openhands_system_stats_contracts`: deprecated V0 status telemetry marker, uptime/idle tracking, psutil process CPU/memory/disk sampling, Linux `/proc/<pid>/io` parsing, and fallback I/O behavior.
- `openhands/app_server/user/README.md` -> `openhands_user_docs_contracts`: user management ownership, user context abstraction, auth compatibility layer, user router and injector roles, authentication/session management, profile retrieval, user-scoped service resolution, and JWT integration.

### OpenHands API-04 user-context contracts, rows 171-175

Assimilated the next OpenHands user-management shard into the control-plane workflow contract:

- `auth_user_context.py` contributes the auth-backed user-context compatibility layer: cached user info from settings, raw/provider-env token views, authenticated git URL resolution, latest-token lookup, custom secret projection, MCP API key retrieval, user git metadata, and request-state injector reuse.
- `skills_router.py` contributes authenticated global/user skill discovery over microagent directories, YAML frontmatter parsing, warning-only parse failures, source/type/trigger metadata, deterministic sorting, and cursor pagination by skill name.
- `specifiy_user_context.py` contributes explicit admin/specified user-context semantics: optional user id, unsupported user-scoped secret/token/profile methods, singleton admin context, request-state override, and fail-closed conflict detection when a non-admin context already exists.
- `user_context.py` contributes the abstract user-operation contract and discriminated injector boundary for user id/profile, authenticated git URLs, provider tokens, latest tokens, custom secrets, MCP API keys, and user git metadata.
- `user_models.py` contributes the settings-derived `UserInfo` profile shape with optional user id and provider-token pagination vocabulary.

API-04 remains in progress after this wave; rows 171-175 are now imported and the next queued OpenHands API-04 shard starts at row 176.

### OpenHands API-04 user/utils contracts, rows 176-180

Assimilated the next OpenHands user and app-server utility shard into the control-plane workflow contract:

- `user_router.py` contributes authenticated `/users/me` and `/users/git-info` behavior, including sandbox-session ownership validation before exposing unmasked secrets, default masked profile responses, 401 handling for missing user/git info, and OpenAPI security dependency signaling.
- `utils/README.md` contributes app-server utility ownership boundaries for UTC date helpers, SQL helpers, dynamic imports, and safe import-error handling.
- `utils/dependencies.py` contributes optional session API-key enforcement, `X-Session-API-Key` fail-closed checks when configured, SAAS `X-Access-Token` documentation signaling without blocking cookie auth, and no-dependency behavior for non-SAAS/no-key modes.
- `utils/docker_utils.py` contributes Docker-aware localhost URL rewriting that preserves ports, paths, query strings, and non-localhost hosts.
- `utils/encryption_key.py` contributes encryption-key modeling, context-gated secret serialization, deterministic `JWT_SECRET` key IDs for multi-pod deployments, existing key-file reload, and generated key persistence with secret exposure context.

API-04 remains in progress after this wave; rows 176-180 are now imported and the next queued OpenHands API-04 shard starts at row 181.

### OpenHands API-04 utility contracts, rows 181-185

Assimilated the next OpenHands app-server utility shard into the control-plane workflow contract:

- `utils/import_utils.py` contributes fully-qualified dynamic imports, runtime implementation substitution, subclass validation, default fallback behavior, and cached implementation resolution.
- `utils/llm_metadata.py` contributes LiteLLM extra-body gating for OpenHands/proxy models and V1 trace metadata with model, type, web host, version, optional session id, and optional user id tags.
- `utils/models.py` contributes the generic edit response envelope used by app-server edit operations.
- `utils/paging_utils.py` contributes opaque URL-safe base64 page IDs, tolerant invalid-token decoding, offset pagination, and next-page token generation.
- `utils/sql_utils.py` contributes SQLAlchemy declarative base ownership, JSON type adaptation with secret exposure context, JWE-backed `SecretStr` persistence, UTC datetime normalization, enum string persistence, and row-to-dict projection.

API-04 remains in progress after this wave; rows 181-185 are now imported and the final queued OpenHands API-04 shard starts at row 186.

### OpenHands API-04 closure and API-05 web-client config contracts, rows 186-190

Assimilated the final OpenHands API-04 row and opened the API-05 web-client/server surface shard in the control-plane workflow contract:

- `v1_router.py` closes API-04 by contributing the canonical `/api/v1` aggregate router boundary and its included event, conversation, pending-message, sandbox, settings, secrets, user, skills, webhook, web-client, git, and config route surfaces.
- `web_client/default_web_client_config_injector.py` contributes environment-derived public web-client configuration: OSS PostHog fallback, OAuth provider discovery, ISO maintenance-window parsing, exact-string feature flags, recaptcha/auth URL exposure, stale-error freshness timestamping, GitHub app slug exposure, and global app-mode merge.
- `web_client/web_client_config_injector.py` contributes the discriminated async injector boundary for public web-client configuration retrieval.
- `web_client/web_client_deployment_mode.py` contributes deployment-mode inference from `OH_WEB_HOST`/`WEB_HOST`, including managed All-Hands/OpenHands cloud domains, self-hosted fallback, and unknown mode for unset hosts.
- `web_client/web_client_models.py` contributes feature-flag defaults, deployment-mode auto-fill validation, and the public web-client configuration model surface.

API-04 is now imported. API-05 is now in progress; rows 187-190 are imported and the next queued OpenHands API-05 shard starts at row 191.

### OpenHands API-05 web-client/server bootstrap contracts, rows 191-195

Assimilated the next OpenHands API-05 web-client and legacy server bootstrap shard into the control-plane workflow contract:

- `web_client/web_client_router.py` contributes the unauthenticated public `/web-client/config` endpoint used by frontend bootstrap, with delegation to the global web-client config injector.
- `server/README.md` contributes the legacy WebSocket server protocol map: action/observation message shapes, initialize/start/read/write/run/browse/think/finish actions, read/browse/run/chat observations, session/agent-session/conversation-manager responsibilities, file operation flow, security analysis forwarding, and inactive-session cleanup.
- `server/__init__.py` contributes an empty package-marker invariant with no public imports or side effects.
- `server/__main__.py` contributes deprecated V0 uvicorn entrypoint behavior: `openhands.server.listen:app`, all-interface binding, env-derived port, DEBUG log-level switch, JSON-log color handling, and explicit V1 migration warning.
- `server/app.py` contributes legacy-compatible FastAPI app composition with MCP mounting, combined lifespans, optional V1 app lifespan service, versioned app metadata, authentication-error 401 normalization, V1 router inclusion, and health router inclusion.

API-05 remains in progress after this wave; rows 191-195 are now imported and the next queued OpenHands API-05 shard starts at row 196.

### OpenHands API-05 legacy server config/data-model contracts, rows 196-200

Assimilated the next OpenHands API-05 server config and data-model shard into the control-plane workflow contract:

- `server/config/server_config.py` contributes legacy V0 server configuration defaults, environment-selected config class loading, store/auth/monitoring implementation class paths, analytics/GitHub/feature flag exposure, V1 enablement, config serialization, and `get_impl`-backed load/verify behavior.
- `server/constants.py` contributes the session-scoped WebSocket room key template (`room:{sid}`) and its legacy V0 boundary marker.
- `server/data_models/agent_loop_info.py` contributes conversation-scoped agent-loop metadata: URL, session API key, event-store reference, default running conversation status, and optional runtime status.
- `server/data_models/conversation_info.py` contributes legacy conversation metadata plus live state fields: runtime status, selected repo/branch/provider, trigger, connection count, URL/session key, timestamps, PR numbers, conversation version, sub-conversations, public flag, sandbox id, and model identity.
- `server/data_models/conversation_info_result_set.py` contributes paginated conversation listing envelopes with default-empty results and optional next-page id.

API-05 remains in progress after this wave; rows 196-200 are now imported and the next queued OpenHands API-05 shard starts at row 201.

### OpenHands API-05 legacy feedback/file/listen contracts, rows 201-205

Assimilated the next OpenHands API-05 legacy server feedback, file, and listen shard into the control-plane workflow contract:

- `server/data_models/feedback.py` contributes feedback payload shape, polarity/backward-compatible feedback-field mirroring, public/private permissions, optional trajectory capture, diagnostic elision for trajectory/token-like data, remote feedback submission, non-200 failure handling, and JSON response decoding.
- `server/file_config.py` contributes upload ignore patterns, safe filename sanitization, upload config sanity checks, extension normalization, wildcard/no-extension handling, and collision-free copy filename generation.
- `server/files.py` contributes upload-files response modeling with exposed file URLs and skipped-file reporting.
- `server/listen.py` contributes legacy ASGI composition: optional frontend static mount, localhost CORS middleware, cache-control middleware, in-memory rate limiting, and Socket.IO wrapping over the FastAPI base app.
- `server/listen_socket.py` contributes backward-compatible `sio` re-export from shared server state for listen/enterprise compatibility.

API-05 remains in progress after this wave; rows 201-205 are now imported and the next queued OpenHands API-05 shard starts at row 206.

### OpenHands API-05 legacy middleware/MCP/service contracts, rows 206-210

Assimilated the next OpenHands API-05 legacy middleware, monitoring, route, and service shard into the control-plane workflow contract:

- `server/middleware.py` contributes localhost-aware CORS fallback, development-mode allow-all warning behavior, asset/non-asset cache control, per-client in-memory request history, soft sleep threshold, hard deny threshold, asset rate-limit exemption, and 429 JSON retry responses.
- `server/monitoring.py` contributes a non-disruptive monitoring extension point with no-op hooks for session events, agent-session startup outcomes, and conversation creation starts.
- `server/routes/mcp.py` contributes masked FastMCP setup, conversation follow-up link injection, PR/MR number extraction and metadata persistence, request-scoped provider-token/access-token/user-id resolution, and provider-specific create tools for GitHub, GitLab, Bitbucket, Bitbucket Data Center, and Azure DevOps.
- `server/routes/public.py` contributes the migrated LLM-model dependency used by the V1 config router and retained for enterprise/SaaS dependency override compatibility.
- `server/services/conversation_service.py` contributes immutable provider-token scaffold creation for configured providers with token and user identity intentionally unset.

API-05 remains in progress after this wave; rows 206-210 are now imported and the final queued OpenHands API-05 shard starts at row 211.

### OpenHands API-05 closure and API-06 server boundary contracts, rows 211-215

The OpenHands control-plane assimilation workflow now includes the final API-05 legacy conversation service contract and the first API-06 server boundary contracts. `conversation_stats.py` contributes a persisted LLM metrics pattern: conversation/user-scoped metrics paths, restore-on-init from a file store, base64-encoded pickle persistence, duplicate restored/active service detection, registry-driven metric transfer, zero-cost metric pruning, and merge-and-save overwrite semantics. In InfRing terms, this should remain evidence for metrics snapshot/recovery semantics, not a recommendation to adopt unsafe pickle persistence at Kernel trust boundaries.

The API-06 imports start the legacy server boundary surface. `settings.py` contributes provider settings and custom-secret DTO shapes, frontend token-status enrichment, redacted secret listings, `SecretStr` value boundaries, and enum-value serialization. `shared.py` contributes singleton bootstrap mechanics: dotenv loading, global config/server-config materialization, file-store construction, optional Redis Socket.IO fanout, ASGI Socket.IO buffer limits, and dynamic monitoring/settings/secrets/conversation store implementation resolution. `static.py` contributes the SPA fallback pattern from static path failure to `index.html`. `types.py` contributes the app-mode vocabulary, server config interface contract, session middleware protocol marker, and typed settings/auth/session error categories.

API-05 is now fully imported. API-06 is now in progress; rows 212-215 are imported and the next queued OpenHands API-06 shard starts at row 216.

### OpenHands API-06 closure and EVENT-01 event boundary start, rows 216-220

The OpenHands control-plane assimilation workflow now includes the remaining API-06 legacy server contracts plus the first EVENT-01 event protocol boundary. `server/user_auth/__init__.py` contributes dependency-injected auth accessors that keep route handlers thin by projecting provider tokens, access tokens, user ids, settings, secrets, backing stores, and auth type from a request-scoped user-auth object. `server/user_auth/default_user_auth.py` contributes default OSS identity semantics: no user id/email/access token, cached settings/secrets stores, config-merged settings loading, secrets-backed provider tokens, no MCP API key, and a root-only compatibility lookup. `server/user_auth/user_auth.py` contributes the pluggable auth interface: cookie/bearer vocabulary, abstract identity/token/settings/secrets methods, request-state instance caching, configured implementation lookup, and provider-token-backed git metadata retrieval.

`server/utils.py` contributes fail-closed conversation utility behavior: conversation ids are bounded to 100 characters and reject null bytes, path traversal characters, slashes/backslashes, and control characters; conversation stores are materialized per current user and cached on request state; unique conversation ids retry with UUID4 hex until absent; and missing metadata maps to HTTP 404. `events/__init__.py` starts EVENT-01 by defining the canonical public event package exports for `Event`, `EventSource`, `EventStream`, `EventStreamSubscriber`, and `RecallType`.

API-06 is now fully imported. EVENT-01 is now in progress; row 220 is imported and the next queued OpenHands EVENT-01 shard starts at row 221.

### OpenHands EVENT-01 action protocol contracts, rows 221-225

The OpenHands control-plane assimilation workflow now captures the first concrete event-action shard. `events/action/__init__.py` defines the public action vocabulary exported from the package boundary, spanning base actions, command/IPython actions, browse actions, file actions, agent lifecycle actions, MCP actions, message actions, confirmation state, security risk, task tracking, and loop recovery. `events/action/action.py` contributes the shared base semantics: actions extend events, default to non-runnable, carry explicit confirmation states, and use an integer security-risk order from unknown through high.

`events/action/agent.py` contributes a dense set of agent-control contracts. State change is a client notification; finish/think/reject/delegate actions each provide deterministic message rendering; recall actions retain recall type and truncated query displays; condensation requires exactly one forget specification mode and paired summary/offset fields, validates both on init and when accessed, and expands ranges inclusively; task tracking renders clear/single/multiple task states; loop recovery carries option-based recovery choices. `events/action/browse.py` contributes runnable URL and interactive browser actions with default unknown risk, optional accessibility tree returns, BrowserGym user-message support, and stable rendering. `events/action/commands.py` contributes runnable shell/IPython actions with confirmation/security defaults, command-input/static/blocking/hidden/cwd metadata, IPython include-extra and kernel-init metadata, and deterministic message/string output.

EVENT-01 remains in progress; rows 221-225 are imported and the next queued OpenHands EVENT-01 shard starts at row 226.

### OpenHands EVENT-01 file, MCP, message, and async event-store contracts, rows 226-230

The OpenHands control-plane assimilation workflow now captures the next event protocol shard. `events/action/empty.py` contributes an explicit null action for intentional no-op events. `events/action/files.py` contributes runnable file read/write/edit contracts: read/write default to whole-file ranges, carry unknown security risk until classified, and expose deterministic user messages; read actions track implementation source and optional OH_ACI view ranges; write actions render path/range/thought/content; edit actions support both OH_ACI command payloads (`create`, `str_replace`, `insert`, `undo_edit`, `write`, and view-as-read) and LLM-based content/range edits with representation switching by implementation source.

`events/action/mcp.py` contributes MCP invocation shape: runnable name plus argument mapping, thought metadata, default unknown security risk, and deterministic rendered text. `events/action/message.py` contributes user and system message surfaces: user messages carry content, file/image attachments, a wait-for-response flag, unknown risk metadata, and a deprecated `images_urls` alias; system messages carry content, optional tool list, default OpenHands version, optional agent class, and are documented as first-event-stream messages. `events/async_event_store_wrapper.py` contributes async access over blocking event-store searches by preserving args/kwargs and yielding events via the running loop executor.

EVENT-01 remains in progress; rows 226-230 are imported and the next queued OpenHands EVENT-01 shard starts at row 231.

### OpenHands EVENT-01 event-store spine contracts, rows 231-235

The OpenHands control-plane assimilation workflow now captures the event-store spine. `events/event.py` contributes the base event metadata contract: source vocabulary (`agent`, `user`, `environment`), file edit/read implementation-source vocabularies, invalid id fallback, optional message/id/timestamp/source/cause/timeout accessors, hard-timeout coupling to a `blocking` attribute when present, LLM metrics metadata, tool-call metadata, and response-id attachment. `events/event_filter.py` contributes composable event filtering by include/exclude types, source value, ISO timestamp bounds, hidden flag, and case-insensitive query search over serialized event JSON.

`events/event_store.py` contributes local file-backed event search: lazy current-id calculation from event filenames, cache pages aligned to cache-size boundaries, forward and reverse search bounds, shutdown-aware iteration, cache-page reads with individual-event fallback, filter and limit enforcement, JSON deserialization, latest-event helpers, source filtering, and warning fallback for invalid filename ids. `events/event_store_abc.py` contributes the canonical abstract store contract plus deprecated wrappers that map old `get_events`, source filtering, and matching-event APIs into `search_events`. `events/nested_event_store.py` contributes remote event pagination with optional `X-Session-API-Key`, 404-as-empty semantics, page limit clamping to 100, forward and reverse cursor maintenance, filter/limit enforcement, and latest/single-event helpers.

EVENT-01 remains in progress; rows 231-235 are imported and the next queued OpenHands EVENT-01 shard starts at row 236.
