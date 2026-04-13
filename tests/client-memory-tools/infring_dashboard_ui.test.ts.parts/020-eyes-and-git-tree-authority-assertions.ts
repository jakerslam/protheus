
  assert.ok(fs.existsSync(eyesPagePath), 'eyes page module missing');
  assertContains(laneSource, "'eyes',", 'dashboard static bundle should include eyes page script');
  assertContains(appSource, "'eyes'", 'router valid pages should include eyes');
  assertContains(htmlSource, "page === 'eyes'", 'eyes page template missing');
  assertContains(htmlSource, 'x-data="eyesPage"', 'eyes page x-data binding missing');
  assertContains(htmlSource, '<span class="nav-label">Eyes</span>', 'eyes sidebar nav entry missing');
  assertContains(laneSource, "if (pathname === '/api/eyes')", 'eyes API route missing');
  assertContains(laneSource, 'upsertManualEye', 'manual eyes upsert helper missing');
  assertContains(laneSource, 'catalog-store-kernel', 'eyes API should sync through catalog-store rust authority');
}

function assertAgentGitTreeAuthority() {
  const laneSource = readUtf8(TARGET_SOURCE);
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  assertContains(
    laneSource,
    "const AGENT_GIT_TREE_KIND_MASTER = 'master';",
    'agent git tree master constant missing'
  );
  assertContains(
    laneSource,
    'function ensureAgentGitTreeAssignments(snapshot, options = {})',
    'agent git tree assignment authority missing'
  );
  assertContains(
    laneSource,
    'git_branch: gitTree.git_branch',
    'agent git branch propagation missing from dashboard API rows'
  );
  assertContains(
    laneSource,
    'workspace_dir: gitTree.workspace_dir',
    'agent workspace propagation missing from dashboard API rows'
  );
  assertContains(
    chatSource,
    'applyAgentGitTreeState(targetAgent, sourceState)',
    'chat git-tree state merge helper missing'
  );
  assertContains(
    chatSource,
    'self.applyAgentGitTreeState(self.currentAgent, data || {});',
    'chat session git-tree sync bridge missing'
  );
  assertContains(
    laneSource,
    'function deleteGitBranchIfSafeForAgent(agentId, profile = null, archivedMeta = null)',
    'agent git branch delete helper missing'
  );
  assertContains(
    laneSource,
    'const branchCleanup = deleteGitBranchIfSafeForAgent(key, profileBeforeDelete, archived);',
    'single archived agent delete must cleanup isolated git branch'
  );
  assertContains(
    laneSource,
    'const branchCleanup = deleteGitBranchIfSafeForAgent(id, candidate.profile, candidate.archived);',
    'bulk archived agent delete must cleanup isolated git branches'
  );
}

function assertInterfaceSafetyGuards() {
  const appSource = readUtf8(APP_STATIC_TS_PATH);
  const agentsSource = readUtf8(AGENTS_PAGE_TS_PATH);
  const hostSource = readUtf8(TARGET);
  const laneSource = readUtf8(TARGET_SOURCE);
  const wsBridgeSource = readUtf8(AGENT_WS_BRIDGE_TS_PATH);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const componentsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/css/components.css'));

  assertContains(
    appSource,
    'getAppStore() {',
    'app shell must include guarded app-store accessor to avoid undefined dereference races'
  );
  assertContains(
    appSource,
    "assistantName: 'Assistant'",
    'app shell must retain assistant identity bootstrap state'
  );
  assertContains(
    appSource,
    'applyBootstrapRuntimeState(statusObj, versionObj);',
    'status polling must hydrate bootstrap runtime metadata into the app store'
  );
  assertContains(
    appSource,
    "agents = await InfringAPI.get('/api/agents?view=sidebar&authority=runtime');",
    'agent sidebar hydration must call authoritative runtime agents endpoint'
  );
  assertContains(
    agentsSource,
    'configFormOriginal: {}',
    'agents page must keep an original config snapshot for safe local mutation'
  );
  assertContains(
    agentsSource,
    'rememberAgentIdentity(agent, extra) {',
    'agents page must cache normalized agent identity metadata'
  );
  assertContains(
    agentsSource,
    'setConfigFormPath(path, value) {',
    'agents page must expose safe config path mutation helpers'
  );
  assertContains(
    agentsSource,
    'var seq = Number(this._lifecycleLoadSeq || 0) + 1;',
    'agents lifecycle loader must ignore stale snapshot responses'
  );
  assertContains(
    laneSource,
    'const strictRuntimeAuthority = runtimeAuthorityRequested === true;',
    'runtime authority sidebar queries must run in strict mode to avoid stale roster poisoning'
  );
  assertContains(
    laneSource,
    'strict: strictRuntimeAuthority,',
    'runtime authority roster fetch must pass strict runtime mode into authoritative resolver'
  );
  assertContains(
    laneSource,
    'Math.max(RUNTIME_AUTHORITY_LANE_TIMEOUT_MS, 1800)',
    'strict sidebar runtime authority timeout floor should avoid transient empty roster flaps'
  );
  assertContains(
    laneSource,
    '!strictRuntimeAuthority &&',
    'runtime authority roster path must not fall back to stale cached sidebar agent rows'
  );
  assertContains(
    laneSource,
    "type: 'agent_purged'",
    'DELETE /api/agents/:id must return idempotent purge response for stale agent IDs'
  );
  assertContains(
    laneSource,
    'zombie_purged: true',
    'stale-agent delete path must mark zombie_purged telemetry for observability'
  );
  assertContains(
    laneSource,
    "function responseLooksTelemetryDump(text) {",
    'strict output contract must include telemetry-dump detector for non-status prompts'
  );
  assertContains(
    laneSource,
    "if (!runtimeTask && !asksForStatus && responseLooksTelemetryDump(normalized)) {",
    'strict output contract must reject telemetry dumps when user did not request status'
  );
  assertContains(
    laneSource,
    'function compatApiPayload(pathname, reqUrl, snapshot)',
    'compat API payload router must remain wired for runtime-aligned responses'
  );
  assertContains(
    laneSource,
    'function runtimeSyncSummary(snapshot)',
    'runtime sync summary helper must remain available for chat/runtime payloads'
  );
  assertContains(
    laneSource,
    'function isPlaceholderResponse(value)',
    'placeholder response guard must remain available to block transcript placeholders'
  );
  assertContains(
    laneSource,
    "if (action === 'app.chat') {",
    'dashboard action lane must keep the app.chat branch'
  );
  assertContains(
    laneSource,
    "runAgentMessage(requestedAgentId, input, latestSnapshot, { allowFallback: true });",
    'runtime chat action must preserve fallback dispatch to avoid stale agent dead-ends'
  );
  assertContains(
    laneSource,
    "type: 'infring_dashboard_runtime_chat'",
    'runtime chat payload type marker missing'
  );
  assertContains(
    laneSource,
    'runtime_sync: turn.runtime_sync || null,',
    'runtime chat response must include runtime_sync passthrough'
  );
  assertContains(
    laneSource,
    'Never output placeholders such as <text response to user> or <answer>.',
    'runtime chat prompt contract must explicitly ban placeholder output'
  );
  assertContains(
    laneSource,
    'Historical memory files are in ${PRIMARY_MEMORY_DIR}/YYYY-MM-DD.md',
    'runtime chat prompt contract must preserve historical-memory guidance'
  );
  assertContains(
    laneSource,
    'Runtime awareness:',
    'runtime chat prompt contract must preserve runtime-awareness guidance'
  );
  assertContains(
    laneSource,
    "rejectedReason === 'telemetry_mismatch'",
    'strict fallback path must recover telemetry mismatch with conversational response'
  );
  assert.ok(
    !appSource.includes('/api/dashboard/snapshot'),
    'app shell must stay thin and must not synthesize agent rows from snapshot fallback payloads'
  );
  assertContains(
    appSource,
    "if (!store || typeof store.refreshAgents !== 'function') throw new Error('app_store_unavailable');",
    'new-agent flow must fail closed when app store is unavailable instead of throwing property-access crashes'
  );
  assertContains(
    appSource,
    "if (msg.indexOf('agent_not_found') >= 0) {",
    'sidebar archive flow must gracefully handle stale agent_not_found responses'
  );
  assertContains(
    appSource,
    '_sidebar_quick_action',
    'chat sidebar search must expose quick action rows for navigation and connection recovery'
  );
  assertContains(
    chatSource,
    'derivePromptSuggestionFallback(agent, hint, gateContext)',
    'chat prompt suggestion fallback helper missing'
  );
  assertContains(
    chatSource,
    'shouldSuspendPointerFx()',
    'chat pointer effects must suspend while voice capture is active'
  );
  assertContains(
    chatSource,
    'Gateway pairing is required. Open Settings, pair this dashboard with the gateway, then retry.',
    'chat send pipeline must normalize pairing guidance instead of dumping raw transport errors'
  );
  assertContains(
    chatSource,
    '**Slash Help**',
    'chat slash help should provide grouped quick-action guidance'
  );
  assertContains(
    appSource,
    "statusAgentCountHint > 0 || connectionState === 'connecting' || connectionState === 'reconnecting'",
    'strict roster refresh should preserve prior agents while runtime still reports active agents'
  );
  assertContains(
    appSource,
    'strict_roster_transient_empty',
    'strict roster transient-empty hold marker missing'
  );
  assertContains(
    appSource,
    'Removed stale agent "',
    'sidebar archive flow must surface stale-agent purge feedback'
  );
  assertContains(
    appSource,
    "if (!store) {\n        this.connected = false;\n        this.connectionState = 'connecting';\n        return;",
    'pollStatus must guard missing app store before reading hydration flags'
  );
  assertContains(
    appSource,
    'runtime.facade_confidence_percent',
    'runtime facade confidence must come from rust authority payload when available'
  );
  assertContains(
    appSource,
    'runtime.facade_eta_seconds',
    'runtime facade eta must come from rust authority payload when available'
  );
  assertContains(
    appSource,
    'runtime.facade_response_p95_ms',
    'runtime facade p95 must come from rust authority payload when available'
  );
  assertContains(
    appSource,
    'dashboardPopupOrigin(overrides) {',
    'dashboard popup base helper missing'
  );
  assertContains(
    appSource,
    'activeDashboardPopupOrigin() {',
    'dashboard popup selector helper missing'
  );
  assertContains(
    appSource,
    'bottomDockPopupOrigin() {',
    'bottom dock popup must route through shared popup origin helper'
  );
  assertContains(
    appSource,
    'dashboardPopupStateOrigin() {',
    'chat-nav and topbar popups must route through the shared popup state origin helper'
  );
  assertContains(
    appSource,
    'showTopbarNavPopup(label, ev) {',
    'topbar chat-nav hover should be authored through the shared popup state'
  );
  assertContains(
    appSource,
    'showDashboardPopup(id, label, ev, overrides) {',
    'shared dashboard popup opener missing'
  );
  assertContains(
    appSource,
    'showCollapsedSidebarAgentPopup(agent, ev) {',
    'collapsed sidebar agent hover should call the shared popup object directly'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showMapItemPopup(msg, idx, $event)\"",
    'chat map items should route hover previews through the shared dashboard popup object'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showMapDayPopup(msg, $event)\"",
    'chat map day markers should route hover previews through the shared dashboard popup object'
  );
  assert.ok(
    !htmlSource.includes('class="chat-map-preview"'),
    'chat map should not keep inline preview popup markup once shared dashboard popup owns that surface'
  );
  assert.ok(
    !componentsSource.includes('.chat-map-preview {'),
    'chat map should not keep bespoke preview popup styling once shared dashboard popup owns that surface'
  );
  assertContains(
    appSource,
    'showCollapsedSidebarNavPopup(label, ev) {',
    'collapsed sidebar nav hover should call the shared popup object directly'
  );
  assertContains(
    appSource,
    'hideDashboardPopupBySource(source) {',
    'shared popup source-specific closer missing'
  );
  assertContains(
    appSource,
    'hideDashboardPopup(rawId) {',
    'shared dashboard popup closer missing'
  );
  assertContains(
    appSource,
    'source: String(config.source || \'\').trim(),',
    'shared dashboard popup state should preserve popup source metadata'
  );
  assertContains(
    appSource,
    'showTopbarUtilityPopup(label, body, ev) {',
    'topbar utility hover should be authored through the shared popup state'
  );
  assert.ok(
    !appSource.includes('showChatNavStatusPopup('),
    'chat-nav should not keep a dedicated popup helper path'
  );
  assert.ok(
    !appSource.includes('showChatNavToolPopup('),
    'chat-nav tool popup should use the shared popup object directly'
  );
  assert.ok(
    !appSource.includes('showChatNavUtilityPopup('),
    'chat-nav utility popup should use the shared popup object directly'
  );
  assert.ok(
    !appSource.includes('collapsedAgentHover'),
    'collapsed sidebar should not keep legacy hover state'
  );
  assert.ok(
    !appSource.includes('collapsedSidebarPopupOrigin('),
    'collapsed sidebar should not keep a dedicated popup origin path'
  );
  assertContains(
    htmlSource,
    'class="dashboard-popup-surface dashboard-preview-surface dashboard-popup-overlay"',
    'shared dashboard popup overlay must literally inherit the base preview surface class'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"if (sidebarCollapsed) showCollapsedSidebarNavPopup('Conversations', $event)\"",
    'collapsed sidebar conversation nav should use the shared sidebar popup helper'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"if (sidebarCollapsed) showCollapsedSidebarAgentPopup(agent, $event)\"",
    'collapsed sidebar agent preview should use the shared sidebar popup helper'
  );
  assertContains(
    htmlSource,
    "@scroll.passive=\"scheduleSidebarScrollIndicators(); hideDashboardPopupBySource('sidebar')\"",
    'collapsed sidebar scroll should clear the shared sidebar popup source'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showTopbarNavPopup('Back', $event)\"",
    'topbar back button should inherit the shared popup object'
  );
  assertContains(
    htmlSource,
    ":aria-disabled=\"!canNavigateBack() ? 'true' : 'false'\"",
    'topbar back button should still allow shared popup hover when navigation is unavailable'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showTopbarNavPopup('Forward', $event)\"",
    'topbar forward button should inherit the shared popup object'
  );
  assertContains(
    htmlSource,
    ":aria-disabled=\"!canNavigateForward() ? 'true' : 'false'\"",
    'topbar forward button should still allow shared popup hover when navigation is unavailable'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showTopbarUtilityPopup('Search', 'Search coming soon', $event)\"",
    'topbar search preview should inherit the shared popup object'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"if (String(agentStatusLabel(agent) || '').trim()) showDashboardPopup('chat-nav-status:' + String(agent.id || ''), 'Agent status', $event, { source: 'chat_nav', side: 'right', body: String(agentStatusLabel(agent) || '').trim(), meta_origin: 'Chat nav', meta_time: formatChatSidebarTime((chatSidebarPreview(agent) || {}).ts) }); else hideDashboardPopup('chat-nav-status:' + String(agent.id || ''))\"",
    'chat-nav status preview should call the shared popup object directly'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"if (String((chatSidebarPreview(agent) || {}).tool_label || '').trim()) showDashboardPopup('chat-nav-tool:' + String(agent.id || ''), 'Tool activity', $event, { source: 'chat_nav', side: 'right', body: String((chatSidebarPreview(agent) || {}).tool_label || '').trim(), meta_origin: 'Chat nav', meta_time: formatChatSidebarTime((chatSidebarPreview(agent) || {}).ts), unread: !!(chatSidebarPreview(agent) || {}).unread_response }); else hideDashboardPopup('chat-nav-tool:' + String(agent.id || ''))\"",
    'chat-nav tool preview should call the shared popup object directly'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showDashboardPopup('chat-nav-utility:conversation-search', 'Conversation search', $event, { source: 'chat_nav', side: 'right', body: 'Search coming soon', meta_origin: 'Chat nav' })\"",
    'chat-nav search affordance should call the shared popup object directly'
  );
  assertContains(
    htmlSource,
    "@mouseenter=\"showDashboardPopup('chat-nav-utility:archive-chat', 'Archive chat', $event, { source: 'chat_nav', side: 'right', body: 'Archive this agent conversation', meta_origin: 'Chat nav' })\"",
    'chat-nav archive affordance should call the shared popup object directly'
  );
  assert.ok(
    !htmlSource.includes(":title=\"chatSidebarPreview(agent).tool_label || 'Tool call'\""),
    'chat-nav tool preview should not fall back to a native title tooltip'
  );
  assert.ok(
    !htmlSource.includes('data-tooltip="Coming soon"'),
    'chat-nav search affordance should not fall back to the old dashboard-preview-trigger tooltip'
  );
  assert.ok(
    !htmlSource.includes('nav-agent-collapsed-hover-float'),
    'legacy sidebar collapsed popup float should be removed once shared popup overlay is active'
  );
  assertContains(
    componentsSource,
    '.dashboard-popup-surface',
    'shared dashboard popup surface styles missing'
  );
  assertContains(
    componentsSource,
    '.dashboard-dropdown-surface {',
    'shared dashboard dropdown surface styles missing'
  );
  assertContains(
    htmlSource,
    'class="topbar-hero-menu dashboard-dropdown-surface"',
    'hero dropdown should share the popup/dropdown surface styling'
  );
  assertContains(
    htmlSource,
    'class="notif-dropdown dashboard-dropdown-surface"',
    'notifications dropdown should share the popup/dropdown surface styling'
  );
  assertContains(
    htmlSource,
    'class="session-dropdown dashboard-dropdown-surface"',
    'session/agent details dropdown should share the popup/dropdown surface styling'
  );
  assertContains(
    htmlSource,
    'class="model-switcher-dropdown model-switcher-dropdown-inline dashboard-dropdown-surface"',
    'model switcher dropdown should share the popup/dropdown surface styling'
  );
  assertContains(
    componentsSource,
    '--dashboard-preview-surface-radius: 18px;',
    'shared dashboard popup surface should use a fixed radius so multi-line cards do not stretch into pills'
  );
  assertContains(
    componentsSource,
    '.dashboard-popup-surface.is-compact {',
    'shared dashboard popup compact state styles missing'
  );
  assertContains(
    componentsSource,
    'text-align: left;',
    'shared dashboard popup compact state should keep the doc-popup text alignment'
  );
  assertContains(
    componentsSource,
    '--dashboard-preview-surface-background: color-mix(in srgb, var(--surface) 84%, transparent);',
    'shared dashboard popup surface should keep an explicit fogged fill instead of near-transparent glass'
  );
  assertContains(
    componentsSource,
    '.dashboard-dropdown-surface {\n  position: relative;\n  isolation: isolate;',
    'shared dashboard dropdown surface should create its own paint layer so menu backgrounds do not render transparent'
  );
  assertContains(
    componentsSource,
    '.dashboard-popup-overlay {',
    'shared dashboard popup overlay positioning styles missing'
  );
  assertContains(
    componentsSource,
    'left: -9999px;',
    'shared dashboard popup overlay should default offscreen to prevent top-corner flash'
  );
  assert.ok(
    !/\.dashboard-popup-surface\s*\{[\s\S]*position:\s*fixed;/.test(componentsSource),
    'shared dashboard popup surface must stay position-agnostic so inherited popups do not jump to the top corner'
  );
  assertContains(
    chatSource,
    "lower === 'i lost the final response handoff for this turn. context is still intact, and i can continue from exactly where this left off.'",
    'failover recovery must not recurse on the pure lost-handoff placeholder sentence'
  );
  assertContains(
    chatSource,
    "lower.indexOf('completed tool steps:') === 0",
    'failover recovery must not treat tool-only completion summaries as backend failures'
  );
  assertContains(
    chatSource,
    'textLooksNoFindingsPlaceholder: function(text)',
    'chat UI must detect no-findings placeholder copy so tool-only web turns can be rewritten to visible summaries'
  );
  assertContains(
    chatSource,
    'textMentionsContextGuard: function(text)',
    'chat UI must detect context-guard truncation copy so oversized tool results degrade visibly instead of disappearing'
  );
  assertContains(
    chatSource,
    'lowSignalWebToolSummary: function(tool)',
    'chat UI must synthesize actionable low-signal web summaries from tool-only completions'
  );
  assertContains(
    chatSource,
    'formatToolAggregateMeta: function(tool)',
    'chat UI must format OpenClaw-style tool meta so web-tool fallbacks mention the actual query/url instead of bare tool names'
  );
  assertContains(
    chatSource,
    'backfillToolRowsFromCompletion: function(rows, payload)',
    'chat UI must synthesize missing tool_result rows from completion receipts so tool-only turns stay visible'
  );
  assertContains(
    chatSource,
    "lower.indexOf('search returned no useful information') >= 0",
    'chat UI must treat raw no-useful-information copy as a low-signal placeholder instead of surfacing it verbatim'
  );

  assertContains(
    hostSource,
    "if (req.method === 'GET' && pathname === '/api/status')",
    'dashboard host must expose compatibility status endpoint for the unified browser UI'
  );
  assertContains(
    hostSource,
    'statusPayloadWithBootStage(flags)',
    'dashboard host status endpoint must return fast boot-stage-aware fallback payloads'
  );
  assertContains(
    hostSource,
    "boot_stage: 'backend_unreachable'",
    'dashboard host status fallback must explicitly surface backend-unreachable boot stage'
  );
  assertContains(
    hostSource,
    "if (req.method === 'GET' && pathname === '/api/config')",
    'dashboard host must expose compatibility config endpoint for the unified browser UI'
  );
  assertContains(
    hostSource,
    "if (req.method === 'GET' && pathname === '/api/auth/check')",
    'dashboard host must expose compatibility auth endpoint for the unified browser UI'
  );
  assertContains(
    hostSource,
    "if (pathname === '/healthz' || pathname.startsWith('/api/')) return void await proxyToBackend(req, res, flags);",
    'dashboard host must proxy health and API requests to the Rust authority lane'
  );
  assertContains(
    hostSource,
    "server.on('upgrade', (req, socket, head) => {",
    'dashboard host must keep websocket upgrade handling for API routes'
  );
  assertContains(
    hostSource,
    'proxyUpgrade(req, socket, head, flags);',
    'dashboard host must proxy websocket upgrades to the Rust authority lane'
  );
  assertContains(
    wsBridgeSource,
    "type: 'phase'",
    'agent websocket bridge should emit phase updates so thinking bubble status stays live'
  );
  assertContains(
    wsBridgeSource,
    "type: 'tool_start'",
    'agent websocket bridge should emit tool_start updates for thought bubble tool transparency'
  );
  assertContains(
    wsBridgeSource,
    "type: 'tool_result'",
    'agent websocket bridge should emit tool_result updates for thought bubble tool transparency'
  );
  assertContains(
    wsBridgeSource,
    'tools: toolRows',
    'agent websocket bridge response payload should include normalized tool cards'
  );

  if (!isRustDashboardLaneWrapperSource(laneSource)) {
    assert.ok(
      /function bodyJson\(req\)[\s\S]*Array\.isArray\(parsed\)/.test(laneSource),
      'server bodyJson must normalize non-object payloads to fail closed'
    );
    assert.ok(
      /parsedPayload[\s\S]*Array\.isArray\(parsedPayload\)[\s\S]*Invalid websocket payload\./.test(laneSource),
      'agent websocket handler must reject non-object payload envelopes'
    );
  }
  assertContains(
    laneSource,
    "const reason = rustTerminationsById.get(id) || '';",
    'contract termination sweeps must be rust-authoritative and not derive local fallback reasons'
  );
  assertContains(
    laneSource,
    "if (authoritySource !== 'rust_runtime_systems') {",
    'runtime swarm execution must fail closed when rust runtime authority is unavailable'
  );
  assertContains(
    laneSource,
    "'rust_unavailable'",
    'runtime recommendation authority metadata must explicitly mark rust authority outages'
  );
  assert.ok(
    !laneSource.includes('idleForMs < AGENT_IDLE_TERMINATION_MS'),
    'idle-cap terminations must be sourced from rust authority payloads, not local idle heuristics'
  );
}

function assertLifecycleAndPlatformSrsEvidence() {
  const laneSource = readUtf8(TARGET_SOURCE);
  const agentsSource = readUtf8(
    path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/agents.ts')
  );
  const htmlSource = readUtf8(
    path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html')
  );
  const readmeSource = readUtf8(path.resolve(ROOT, 'README.md'));
  const initiativeSource = readUtf8(path.resolve(ROOT, 'core/layer2/execution/src/initiative.rs'));
  const importanceSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/importance.rs'));
  const attentionQueueSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/attention_queue.rs'));
  const attentionQueuePartsDir = path.resolve(ROOT, 'core/layer0/ops/src/attention_queue_parts');
  const attentionQueuePartsSource = fs.existsSync(attentionQueuePartsDir)
    ? fs
        .readdirSync(attentionQueuePartsDir, { withFileTypes: true })
        .filter((entry) => entry && entry.isFile() && /\.rs$/i.test(entry.name))
        .map((entry) => fs.readFileSync(path.join(attentionQueuePartsDir, entry.name), 'utf8'))
        .join('\n')
    : '';
  const opsCargoSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/Cargo.toml'));

  // V6-AGENT-LIFECYCLE-001.1
  assertContains(laneSource, 'function deriveAgentContract(', 'V6-AGENT-LIFECYCLE-001.1 missing derive contract helper');
  assertContains(laneSource, 'termination_condition', 'V6-AGENT-LIFECYCLE-001.1 missing termination condition contract field');
  assertContains(laneSource, 'expiry_seconds', 'V6-AGENT-LIFECYCLE-001.1 missing expiry seconds contract field');

  // V6-AGENT-LIFECYCLE-001.3
  assertContains(laneSource, "if (req.method === 'POST' && parts[3] === 'revive')", 'V6-AGENT-LIFECYCLE-001.3 missing revive API route');
  assertContains(agentsSource, "/api/agents/' + encodeURIComponent(agentId) + '/revive'", 'V6-AGENT-LIFECYCLE-001.3 missing revive client call');

  // V6-AGENT-LIFECYCLE-001.4
  assertContains(laneSource, 'function detectContractViolation(', 'V6-AGENT-LIFECYCLE-001.4 missing contract violation detector');
  assertContains(laneSource, 'AGENT_ROGUE_MESSAGE_RATE_MAX_PER_MIN', 'V6-AGENT-LIFECYCLE-001.4 missing rogue rate guard');
  assertContains(laneSource, "error: 'agent_contract_terminated'", 'V6-AGENT-LIFECYCLE-001.4 missing violation termination error path');

  // V6-AGENT-LIFECYCLE-001.5
  assert.ok(
    htmlSource.includes('Recently Terminated') || htmlSource.includes('Archived'),
    'V6-AGENT-LIFECYCLE-001.5 missing archived agents UI surface'
  );
  assertContains(agentsSource, 'formatAgentContractLine(agent)', 'V6-AGENT-LIFECYCLE-001.5 missing contract summary formatter');
  assertContains(agentsSource, 'async reviveTerminated(entry)', 'V6-AGENT-LIFECYCLE-001.5 missing revive action handler');

  // V7-PLATFORM-001.1
  assert.ok(fs.existsSync(path.resolve(ROOT, 'LICENSE')), 'V7-PLATFORM-001.1 missing LICENSE');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'LICENSE-INFRING-NC-1.0')), 'V7-PLATFORM-001.1 missing LICENSE-INFRING-NC-1.0');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'LICENSE-APACHE-2.0')), 'V7-PLATFORM-001.1 missing LICENSE-APACHE-2.0');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'LICENSE_SCOPE.md')), 'V7-PLATFORM-001.1 missing LICENSE_SCOPE.md');
  assertContains(readmeSource, 'license-dual', 'V7-PLATFORM-001.1 missing dual-license README marker');

  // V7-PLATFORM-001.2
  assert.ok(fs.existsSync(path.resolve(ROOT, 'roadmap.md')), 'V7-PLATFORM-001.2 missing roadmap.md');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'glossary.md')), 'V7-PLATFORM-001.2 missing glossary.md');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'apps/sovereign-memory-os')), 'V7-PLATFORM-001.2 missing sovereign-memory-os app scaffold');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'apps/local-research-agent')), 'V7-PLATFORM-001.2 missing local-research-agent app scaffold');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'apps/mcu-sensor-monitor-tiny-max')), 'V7-PLATFORM-001.2 missing mcu-sensor-monitor-tiny-max app scaffold');

  // V7-PLATFORM-001.3
  assert.ok(fs.existsSync(path.resolve(ROOT, 'docs/client/architecture/layer2_initiative_extensions.md')), 'V7-PLATFORM-001.3 missing layer2 initiative extension doc');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'docs/example.md')), 'V7-PLATFORM-001.3 missing docs/example.md');
  assertContains(initiativeSource, 'Layer 2 initiative primitive.', 'V7-PLATFORM-001.3 missing initiative primitive documentation context');

  // V7-PLATFORM-001.4
  assert.ok(fs.existsSync(path.resolve(ROOT, 'docs/plugins/PLUGIN_WASM_COMPONENT_SPEC.md')), 'V7-PLATFORM-001.4 missing plugin WASM spec');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'adapters/protocol/wasm_adapter_skeleton/wit/infring_plugin.wit')), 'V7-PLATFORM-001.4 missing WIT interface skeleton');
  assert.ok(fs.existsSync(path.resolve(ROOT, 'adapters/protocol/wasm_adapter_skeleton/src/lib.rs')), 'V7-PLATFORM-001.4 missing adapter skeleton rust entrypoint');

  // V7-PLATFORM-001.5
  assertContains(opsCargoSource, 'proptest = "1.5"', 'V7-PLATFORM-001.5 missing proptest dependency');
  assertContains(importanceSource, 'proptest!', 'V7-PLATFORM-001.5 missing importance proptest coverage');
  assert.ok(
    attentionQueueSource.includes('proptest!') || attentionQueuePartsSource.includes('proptest!'),
    'V7-PLATFORM-001.5 missing attention queue proptest coverage'
  );

  // V7-PLATFORM-001.6
  assert.ok(fs.existsSync(path.resolve(ROOT, 'docs/observability/OTLP_INTEGRATION_PLAN.md')), 'V7-PLATFORM-001.6 missing OTLP integration plan');

  // V7-PLATFORM-001.7
  assert.ok(
    fs.existsSync(path.resolve(ROOT, 'docs/client/architecture/pure_mode_local_llm_adapters.md')),
    'V7-PLATFORM-001.7 missing pure mode local LLM adapter spec'
  );
}

function runSnapshotAssertions() {
  assertDashboardFileSizeCaps();
  assertSvelteKitPrimaryDashboardContract();
  assertThinClientAuthorityBoundary();
  assertLegacyDashboardArtifactsRemoved();
  assertChatSyntaxGuards();
  assertDashboardInlineScriptsParse();
  assertDashboardBuildVersionFresh();
  assertDashboardVersionRefreshUsesApiVersion();
  assertTopbarHeroSystemMenu();
  assertDashboardHostOverlaysLiveVersion();
  assertChatEnhancementFeatures();
  assertMemoryApiWired();
  assertEyesPageWired();
  assertAgentGitTreeAuthority();
  assertInterfaceSafetyGuards();
  assertLifecycleAndPlatformSrsEvidence();
  const proc = runSnapshot();
  assert.strictEqual(proc.status, 0, `snapshot command failed: ${proc.stderr || proc.stdout}`);

  const payload = parseJson(proc.stdout);
  assert.strictEqual(payload.type, 'infring_dashboard_snapshot');
  assert.ok(
    payload &&
      payload.metadata &&
      (payload.metadata.authority === 'rust_core_lanes' ||
        payload.metadata.authority === 'rust_core_cached_runtime_state'),
    `unexpected dashboard authority: ${payload && payload.metadata ? payload.metadata.authority : '<missing>'}`
  );
  assert.ok(payload.health && typeof payload.health === 'object', 'health payload missing');
  assert.ok(payload.app && typeof payload.app === 'object', 'app payload missing');
  assert.ok(payload.collab && typeof payload.collab === 'object', 'collab payload missing');
  assert.ok(payload.skills && typeof payload.skills === 'object', 'skills payload missing');
  assert.ok(Array.isArray(payload.receipts.recent), 'receipts.recent should be an array');
  assert.ok(Array.isArray(payload.logs.recent), 'logs.recent should be an array');
  assert.ok(Array.isArray(payload.memory.entries), 'memory.entries should be an array');
  assert.ok(typeof payload.receipt_hash === 'string' && payload.receipt_hash.length > 20);
  assert.ok(fs.existsSync(SNAPSHOT_PATH), 'snapshot receipt file missing');

  const onDisk = JSON.parse(fs.readFileSync(SNAPSHOT_PATH, 'utf8'));
  assert.strictEqual(onDisk.type, 'infring_dashboard_snapshot');
  assert.ok(
    typeof onDisk.receipt_hash === 'string' && onDisk.receipt_hash.length > 20,
    'on-disk snapshot should include a receipt hash'
  );
  return payload;
}

function assertContract008() {
  const laneSource = readUtf8(TARGET_SOURCE);
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  assertContains(
    laneSource,
    "pathname === '/api/route/auto' || pathname === '/route/auto'",
    'auto-route endpoint route guard missing'
  );
  assertContains(
    laneSource,
    'const route = planAutoRoute(input, latestSnapshot, {',
    'auto-route endpoint planner call missing'
  );
  assertContains(
    laneSource,
    "authority: 'rust_model_router'",
    'rust-authoritative auto-route marker missing'
  );
  assertContains(
