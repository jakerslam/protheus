    laneSource,
    "'model-router',",
    'auto-route model-router lane call missing'
  );
  assertContains(
    laneSource,
    'fallback.authority = \'ts_heuristic_fallback\';',
    'auto-route ts fallback authority marker missing'
  );
  assertContains(
    laneSource,
    'auto_route: turn.auto_route || null,',
    'turn auto-route metadata propagation missing'
  );
  assertContains(
    laneSource,
    'routed_model: autoRoutePayload.model,',
    'lane payload routed model binding missing'
  );
  assertContains(
    chatSource,
    "var result = await InfringAPI.post('/api/route/auto', {",
    'chat preflight auto-route request missing'
  );
  assertContains(
    chatSource,
    "var prefix = provider ? ('Auto -> ' + provider + '/' + shortModel) : ('Auto -> ' + shortModel);",
    'chat auto-route metadata formatting missing'
  );
  assertContains(
    chatSource,
    "formatAutoModelSwitchLabel(modelId)",
    'auto model switch label formatter missing'
  );
  assertContains(
    chatSource,
    "Model switched from ' + previous + ' to ' + next",
    'auto-route model switch notice copy missing'
  );
  assertContains(
    chatSource,
    "_pendingAutoModelSwitchBaseline: ''",
    'pending auto-switch baseline state missing'
  );
  assertContains(
    chatSource,
    "this._pendingAutoModelSwitchBaseline = this.captureAutoModelSwitchBaseline();",
    'send-path auto-switch baseline capture missing'
  );
  assertContains(
    chatSource,
    "var wsAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();",
    'ws response auto-switch baseline restore missing'
  );
  assertContains(
    chatSource,
    "var httpAutoSwitchPrevious = String(this._pendingAutoModelSwitchBaseline || '').trim();",
    'http response auto-switch baseline restore missing'
  );
  assertContains(
    chatSource,
    "this.maybeAddAutoModelSwitchNotice(httpAutoSwitchPrevious, httpRoute || preflightRoute);",
    'http response auto-switch notice emission missing'
  );
  assertContains(
    chatSource,
    "this._pendingAutoModelSwitchBaseline = '';",
    'auto-switch baseline clear missing'
  );
}

function assertContract009() {
  const laneSource = readUtf8(TARGET_SOURCE);
  assertContains(
    laneSource,
    "assistant_role: 'Agent'",
    'runtime-task acceptance message should be agent-origin for mixed-origin stacking'
  );
  assertContains(
    laneSource,
    'user_system_origin: cleanText(source, 120) || \'runtime_task\'',
    'runtime-task system-origin metadata missing'
  );
  assertContains(
    laneSource,
    'agent_id: turn.agent_id || agentId,',
    'ws/http response agent id metadata missing'
  );
  assertContains(
    laneSource,
    'agent_name: cleanText(turn && turn.agent && turn.agent.name ? turn.agent.name : \'\', 120),',
    'ws/http response agent name metadata missing'
  );
}

function assertContract007() {
  const laneSource = readUtf8(TARGET_SOURCE);
  assertContains(
    laneSource,
    '--system-id=V6-DASHBOARD-007.1',
    'dashboard runtime authority lane system id missing'
  );
  assertContains(
    laneSource,
    'dashboard_runtime_authority',
    'dashboard runtime authority specific check binding missing'
  );
  assertContains(
    laneSource,
    "authority: 'rust_runtime_systems'",
    'dashboard runtime authority rust marker missing'
  );
  assertContains(
    laneSource,
    'attention_drain_required',
    'dashboard runtime authority drain recommendation binding missing'
  );
  assertContains(
    laneSource,
    'attention_compact_required',
    'dashboard runtime authority compact recommendation binding missing'
  );
  assertContains(
    laneSource,
    'throttle_max_depth',
    'dashboard runtime authority throttle depth binding missing'
  );
  assertContains(
    laneSource,
    'memory_resume_eligible',
    'dashboard runtime authority memory resume binding missing'
  );
  assertContains(
    laneSource,
    'maybeApplyRuntimeThrottle(runtime, recommendation.team || DEFAULT_TEAM, recommendation)',
    'queue throttle should consume rust runtime authority recommendation'
  );
  assertContains(
    laneSource,
    'const queueDrain = maybeDrainAttentionQueue(runtime, recommendation);',
    'attention queue drain should consume rust runtime authority recommendation'
  );
  assertContains(
    laneSource,
    "const autoheal = maybeRunAutonomousSelfHeal('interval');",
    'runtime interval loop must invoke autonomous self-heal'
  );
  assertContains(
    laneSource,
    'staleRawMaintenance ||',
    'conduit auto-heal should include stale-raw maintenance trigger'
  );
  assertContains(
    laneSource,
    'const staleLaneGc =',
    'autonomous self-heal conduit-only path should include stale-lane gc branch'
  );
  assertContains(
    laneSource,
    "maybeHealCoarseSignal(latestSnapshot, runtime, flags.team || DEFAULT_TEAM)",
    'autonomous self-heal conduit-only path should run coarse stale-lane remediation'
  );
  assertContains(
    laneSource,
    "policy: staleLaneGc && staleLaneGc.required",
    'conduit-only self-heal policy should expose when stale-lane gc is bundled'
  );
  assertContains(
    laneSource,
    'function shouldSurfaceRuntimeTaskInChat(source = \'\')',
    'runtime task chat-surface policy helper missing'
  );
  assertContains(
    laneSource,
    'const RUNTIME_TASK_CHAT_DEDUPE_MS = 5 * 60 * 1000;',
    'runtime task dedupe window constant missing'
  );
  assertContains(
    laneSource,
    'surfaced_in_chat: surfacedInChat,',
    'runtime task queue result should report chat surfacing status'
  );
  assert.ok(
    !laneSource.includes('Task accepted. Report findings in this thread with receipt-backed evidence.'),
    'runtime task queue should not inject synthetic task-accepted chat messages'
  );
  assertContains(
    laneSource,
    'criticalAttentionOverload',
    'runtime authority should react to critical attention overload'
  );
  assertContains(
    laneSource,
    'cockpit_stale_blocks_raw: staleRawBlocks,',
    'runtime recommendation payload should surface raw stale cockpit blocks'
  );
  assertContains(
    laneSource,
    'critical_attention_overload: criticalAttentionOverload,',
    'runtime recommendation payload should surface critical overload marker'
  );
}

function runContract(contract) {
  runSnapshotAssertions();
  if (contract === 'V6-DASHBOARD-007.1') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.2') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.3') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.4') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.5') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.6') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.7') return assertContract007();
  if (contract === 'V6-DASHBOARD-007.8') return assertContract007();
  if (contract === 'V6-DASHBOARD-008.1') return assertContract008();
  if (contract === 'V6-DASHBOARD-008.2') return assertContract008();
  if (contract === 'V6-DASHBOARD-008.3') return assertContract008();
  if (contract === 'V6-DASHBOARD-008.4') return assertContract008();
  if (contract === 'V6-DASHBOARD-009.1') return assertContract009();
  if (contract === 'V6-DASHBOARD-009.2') return assertContract009();
  assert.fail(`unsupported_contract:${contract}`);
}

function assertNativeChatRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const runtimeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/chat.ts'));
  const componentSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ChatPage.svelte'));
  const composerSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ChatComposer.svelte'));
  const drawerSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ChatDrawer.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/chat/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/chat/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'chat'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark chat as a native route'
  );
  assertContains(
    runtimeSource,
    '/api/agents?view=sidebar&authority=runtime',
    'native chat should load the authoritative sidebar roster'
  );
  assertContains(
    runtimeSource,
    "/api/agents/${encodeURIComponent(agentId)}/session",
    'native chat should read authoritative agent session payloads'
  );
  assertContains(
    runtimeSource,
    "/api/agents/${encodeURIComponent(agentId)}/message",
    'native chat should send messages through the existing agent message endpoint'
  );
  assertContains(
    runtimeSource,
    "mission: 'Fresh chat initialization'",
    'native chat draft creation should preserve the fresh chat initialization contract'
  );
  assertContains(runtimeSource, '/api/models', 'native chat runtime should read the authoritative model catalog for drawer controls');
  assertContains(runtimeSource, '/session/compact', 'native chat runtime should expose compact session authority');
  assertContains(runtimeSource, '/session/reset', 'native chat runtime should expose reset session authority');
  assertContains(runtimeSource, '/stop', 'native chat runtime should expose stop-agent authority');
  assertContains(runtimeSource, '/upload', 'native chat runtime should expose attachment upload through the authoritative upload endpoint');
  assertContains(runtimeSource, 'new WebSocket(url);', 'native chat runtime should own a direct websocket bridge for streaming turns');
  assertContains(runtimeSource, "type: 'message'", 'native chat websocket bridge should send the canonical message envelope');
  assertContains(
    componentSource,
    'const rows = await readSidebarAgents();',
    'native chat page should hydrate from the authoritative roster helper'
  );
  assertContains(
    componentSource,
    'const session = await readAgentSession(agentId);',
    'native chat page should load the authoritative transcript helper'
  );
  assertContains(
    componentSource,
    'await sendAgentMessage(activeAgentId, raw);',
    'native chat page should send through the authoritative chat helper'
  );
  assertContains(
    componentSource,
    'const created = await createDraftAgent();',
    'native chat page should support native draft-chat creation'
  );
  assertContains(componentSource, 'bindStream(agentId);', 'native chat page should bind the authoritative websocket stream when selecting an agent');
  assertContains(componentSource, 'await uploadPendingFiles(activeAgentId, files)', 'native chat page should upload attachments before sending through the native route');
  assertContains(componentSource, 'streamController.sendMessage(finalText, uploadSummary.uploaded)', 'native chat page should prefer websocket send for live streaming when connected');
  assertContains(componentSource, 'await sendAgentMessage(activeAgentId, finalText, uploadSummary.uploaded);', 'native chat page should preserve authoritative HTTP fallback when websocket send is unavailable');
  assertContains(componentSource, '<ChatComposer', 'native chat page should render a dedicated native composer surface');
  assertContains(componentSource, '<ChatDrawer', 'native chat page should render a dedicated native operator drawer');
  assertContains(
    componentSource,
    "href={dashboardClassicHref('chat')}",
    'native chat should preserve a classic escape hatch while advanced legacy features remain'
  );
  assertContains(composerSource, 'input type="file"', 'native composer should expose file attachment selection');
  assertContains(composerSource, "dispatch('submit')", 'native composer should dispatch submit events instead of owning authority directly');
  assertContains(drawerSource, "dispatch('savemodel')", 'native drawer should expose model switching via event dispatch');
  assertContains(drawerSource, "dispatch('compact')", 'native drawer should expose compact session control');
  assertContains(drawerSource, "dispatch('reset')", 'native drawer should expose reset session control');
  assertContains(drawerSource, "dispatch('stop')", 'native drawer should expose stop-agent control');
  assertContains(
    routeSource,
    '<ChatPage />',
    'native chat route should render the Svelte chat page directly'
  );
  assertContains(
    routeLoadSource,
    'export const prerender = true;',
    'native chat route should keep static prerender options in +page.ts'
  );
}

function assertNativeAgentsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const agentsRuntimeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/agents.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/AgentsPage.svelte'));
  const detailSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/AgentDetailPanel.svelte'));
  const templatesSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/AgentTemplatesPanel.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/agents/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/agents/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'agents'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark agents as a native route'
  );
  assertContains(
    agentsRuntimeSource,
    '/api/agents/terminated',
    'native agents runtime should read the authoritative terminated-agent lane'
  );
  assertContains(
    agentsRuntimeSource,
    '/api/templates',
    'native agents runtime should read the authoritative template catalog'
  );
  assertContains(
    agentsRuntimeSource,
    "/api/agents/${encodeURIComponent(agent.id)}",
    'native agents runtime should archive agents through the authoritative agent endpoint'
  );
  assertContains(
    agentsRuntimeSource,
    '/history',
    'native agents runtime should expose authoritative history clearing'
  );
  assertContains(
    agentsRuntimeSource,
    '/clone',
    'native agents runtime should expose authoritative cloning'
  );
  assertContains(
    pageSource,
    'await readSidebarAgents();',
    'native agents page should load the authoritative active roster'
  );
  assertContains(
    pageSource,
    'await readTerminatedAgents();',
    'native agents page should load the authoritative archived-agent roster'
  );
  assertContains(
    pageSource,
    'await readTemplates();',
    'native agents page should load the authoritative template catalog'
  );
  assertContains(
    pageSource,
    'const created = await createDraftAgent();',
    'native agents page should support native draft creation'
  );
  assertContains(
    pageSource,
    'await spawnTemplateAgent(templateName);',
    'native agents page should spawn agents from the existing template manifest contract'
  );
  assertContains(
    pageSource,
    'await updateAgentConfig(selectedAgent.id, { name: nameDraft.trim() });',
    'native agents page should rename agents through the authoritative config patch path'
  );
  assertContains(
    pageSource,
    'await updateAgentModel(selectedAgent.id, modelDraft.trim());',
    'native agents page should switch models through the authoritative model path'
  );
  assertContains(pageSource, '<AgentDetailPanel', 'native agents page should render a dedicated native detail panel');
  assertContains(pageSource, '<AgentTemplatesPanel', 'native agents page should render a dedicated native template panel');
  assertContains(
    pageSource,
    "href={dashboardClassicHref('agents')}",
    'native agents should preserve a classic escape hatch while deeper legacy tabs remain'
  );
  assertContains(detailSource, "dispatch('savename')", 'native agents detail panel should expose rename via event dispatch');
  assertContains(detailSource, "dispatch('savemodel')", 'native agents detail panel should expose model switch via event dispatch');
  assertContains(detailSource, "dispatch('clone')", 'native agents detail panel should expose clone via event dispatch');
  assertContains(detailSource, "dispatch('archive')", 'native agents detail panel should expose archive via event dispatch');
  assertContains(templatesSource, "dispatch('spawn'", 'native agents templates panel should expose spawn via event dispatch');
  assertContains(routeSource, '<AgentsPage />', 'native agents route should render the Svelte agents page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native agents route should keep static prerender options in +page.ts');
}

function assertNativeSettingsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const settingsRuntimeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/settings.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/SettingsPage.svelte'));
  const providersSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ProviderSettingsPanel.svelte'));
  const modelsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ModelCatalogPanel.svelte'));
  const systemInfoSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/SystemInfoPanel.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/settings/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/settings/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'settings'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark settings as a native route'
  );
  assertContains(settingsRuntimeSource, '/api/providers', 'native settings runtime should read the authoritative provider catalog');
  assertContains(settingsRuntimeSource, '/api/models', 'native settings runtime should read the authoritative model catalog');
  assertContains(settingsRuntimeSource, '/api/version', 'native settings runtime should read the authoritative version lane');
  assertContains(settingsRuntimeSource, '/api/status', 'native settings runtime should read the authoritative runtime status lane');
  assertContains(settingsRuntimeSource, '/api/providers/${encodeURIComponent(providerId)}/key', 'native settings runtime should update provider keys through the authoritative key endpoint');
  assertContains(settingsRuntimeSource, '/api/providers/${encodeURIComponent(providerId)}/test', 'native settings runtime should test providers through the authoritative provider test endpoint');
  assertContains(settingsRuntimeSource, '/api/providers/${encodeURIComponent(providerId)}/url', 'native settings runtime should update local provider URLs through the authoritative URL endpoint');
  assertContains(settingsRuntimeSource, '/api/models/custom', 'native settings runtime should manage custom models through the authoritative custom-model contract');
  assertContains(pageSource, 'readProviders()', 'native settings page should load providers through the native settings runtime helper');
  assertContains(pageSource, 'readSettingsModels()', 'native settings page should load the model catalog through the native settings runtime helper');
  assertContains(pageSource, 'readSystemInfo()', 'native settings page should load runtime status through the native settings runtime helper');
  assertContains(pageSource, 'await saveProviderKey(providerId, value)', 'native settings page should save provider keys through the authoritative helper');
  assertContains(pageSource, 'await saveProviderUrl(providerId, value)', 'native settings page should save provider URLs through the authoritative helper');
  assertContains(pageSource, 'await addCustomModel({', 'native settings page should add custom models through the authoritative helper');
  assertContains(pageSource, 'await deleteCustomModel(modelId)', 'native settings page should delete custom models through the authoritative helper');
  assertContains(pageSource, '<ProviderSettingsPanel', 'native settings page should render a dedicated native provider settings panel');
  assertContains(pageSource, '<ModelCatalogPanel', 'native settings page should render a dedicated native model catalog panel');
  assertContains(pageSource, '<SystemInfoPanel', 'native settings page should render a dedicated native system info panel');
  assertContains(pageSource, "href={dashboardClassicHref('settings')}", 'native settings should preserve a classic escape hatch while deeper legacy tabs remain');
  assertContains(providersSource, "dispatch('savekey'", 'native provider settings panel should expose key save via event dispatch');
  assertContains(providersSource, "dispatch('testprovider'", 'native provider settings panel should expose provider test via event dispatch');
  assertContains(providersSource, "dispatch('saveurl'", 'native provider settings panel should expose local URL save via event dispatch');
  assertContains(modelsSource, "dispatch('addcustom')", 'native model catalog panel should expose custom-model creation via event dispatch');
  assertContains(modelsSource, "dispatch('deletecustom'", 'native model catalog panel should expose custom-model deletion via event dispatch');
  assertContains(systemInfoSource, 'formatUptime', 'native system info panel should format runtime uptime locally without new authority');
  assertContains(routeSource, '<SettingsPage />', 'native settings route should render the Svelte settings page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native settings route should keep static prerender options in +page.ts');
}

function assertNativeRuntimeRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const runtimeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/runtime.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/RuntimePage.svelte'));
  const overviewSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/RuntimeOverviewPanel.svelte'));
  const providersSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/RuntimeProvidersPanel.svelte'));
  const webSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/RuntimeWebToolingPanel.svelte'));
  const hostSource = readUtf8(path.resolve(ROOT, 'adapters/runtime/infring_dashboard.ts'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/runtime/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/runtime/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'runtime'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark runtime as a native route'
  );
  assertContains(runtimeSource, '/api/status', 'native runtime helper should read the authoritative runtime status lane');
  assertContains(runtimeSource, '/api/version', 'native runtime helper should read the authoritative version lane');
  assertContains(runtimeSource, '/api/providers', 'native runtime helper should read the authoritative provider lane');
  assertContains(runtimeSource, '/api/agents', 'native runtime helper should read the authoritative agent roster lane');
  assertContains(runtimeSource, '/api/web/status', 'native runtime helper should read the authoritative web tooling status lane');
  assertContains(runtimeSource, '/api/web/receipts?limit=5', 'native runtime helper should read recent web tooling receipts');
  assertContains(runtimeSource, '/api/runtime/policy-debt', 'native runtime helper should read operator-facing policy debt telemetry');
  assertContains(runtimeSource, '/api/runtime/orchestration-surface', 'native runtime helper should read operator-facing orchestration telemetry');
  assertContains(hostSource, '/api/runtime/policy-debt', 'dashboard host should expose native policy debt telemetry to the runtime page');
  assertContains(hostSource, '/api/runtime/orchestration-surface', 'dashboard host should expose native orchestration telemetry to the runtime page');
  assertContains(pageSource, 'await readRuntimePageData();', 'native runtime page should load the bounded runtime slice through the runtime helper');
  assertContains(pageSource, '<RuntimeOverviewPanel', 'native runtime page should render a dedicated native runtime overview panel');
  assertContains(pageSource, '<RuntimeProvidersPanel', 'native runtime page should render a dedicated native provider status panel');
  assertContains(pageSource, '<RuntimeWebToolingPanel', 'native runtime page should render a dedicated native web tooling panel');
  assertContains(pageSource, 'Operator debt', 'native runtime page should expose classic-route debt telemetry to operators');
  assertContains(pageSource, 'Orchestration surface', 'native runtime page should expose orchestration contract telemetry to operators');
  assertContains(pageSource, "href={dashboardClassicHref('runtime')}", 'native runtime should preserve a classic escape hatch while deeper legacy tabs remain');
  assertContains(overviewSource, 'formatUptime', 'native runtime overview panel should format uptime locally without new authority');
  assertContains(providersSource, 'Provider health', 'native runtime providers panel should keep provider health visible in the Svelte route');
  assertContains(webSource, 'formatReceiptTime', 'native runtime web tooling panel should render recent receipt timing locally');
  assertContains(routeSource, '<RuntimePage />', 'native runtime route should render the Svelte runtime page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native runtime route should keep static prerender options in +page.ts');
}

function assertNativeApprovalsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const approvalsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/approvals.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ApprovalsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/approvals/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/approvals/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'approvals'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark approvals as a native route'
  );
  assertContains(approvalsSource, '/api/approvals', 'native approvals runtime should read the authoritative approvals queue');
  assertContains(approvalsSource, '/approve', 'native approvals runtime should approve through the authoritative approval endpoint');
  assertContains(approvalsSource, '/reject', 'native approvals runtime should reject through the authoritative approval endpoint');
  assertContains(pageSource, 'await readApprovals();', 'native approvals page should load approvals through the runtime helper');
  assertContains(pageSource, 'await approveApproval(id)', 'native approvals page should approve through the helper');
  assertContains(pageSource, 'await rejectApproval(id)', 'native approvals page should reject through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('approvals')}", 'native approvals should preserve a classic escape hatch');
  assertContains(routeSource, '<ApprovalsPage />', 'native approvals route should render the Svelte approvals page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native approvals route should keep static prerender options in +page.ts');
}

function assertNativeAnalyticsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const analyticsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/analytics.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/AnalyticsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/analytics/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/analytics/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'analytics'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark analytics as a native route'
  );
  assertContains(analyticsSource, '/api/usage/summary', 'native analytics helper should read the authoritative usage summary lane');
  assertContains(analyticsSource, '/api/usage/by-model', 'native analytics helper should read the authoritative per-model usage lane');
  assertContains(analyticsSource, '/api/usage', 'native analytics helper should read the authoritative per-agent usage lane');
  assertContains(analyticsSource, '/api/usage/daily', 'native analytics helper should read the authoritative daily usage lane');
  assertContains(pageSource, 'await readAnalyticsSnapshot();', 'native analytics page should load its data through the analytics helper');
  assertContains(pageSource, "href={dashboardClassicHref('analytics')}", 'native analytics should preserve a classic escape hatch');
  assertContains(routeSource, '<AnalyticsPage />', 'native analytics route should render the Svelte analytics page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native analytics route should keep static prerender options in +page.ts');
}

function assertNativeLogsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const logsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/logs.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/LogsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/logs/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/logs/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'logs'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark logs as a native route'
  );
  assertContains(logsSource, '/api/audit/recent?n=200', 'native logs helper should read the authoritative recent audit lane');
  assertContains(logsSource, '/api/audit/verify', 'native logs helper should read the authoritative audit verification lane');
  assertContains(pageSource, 'await refresh();', 'native logs page should load its bounded audit slice on mount');
  assertContains(pageSource, 'setInterval(() => void refreshLogsOnly(), 4000)', 'native logs page should keep a bounded polling refresh for recent audit entries');
  assertContains(pageSource, "href={dashboardClassicHref('logs')}", 'native logs should preserve a classic escape hatch');
  assertContains(routeSource, '<LogsPage />', 'native logs route should render the Svelte logs page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native logs route should keep static prerender options in +page.ts');
}

function assertNativeWorkflowsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const workflowsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/workflows.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/WorkflowsPage.svelte'));
  const editorSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/WorkflowEditorPanel.svelte'));
  const runSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/WorkflowRunPanel.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/workflows/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/workflows/+page.ts'));

  assert.ok(
    /\{\s*key:\s*'workflows'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource),
    'dashboard registry should mark workflows as a native route'
  );
  assertContains(workflowsSource, '/api/workflows', 'native workflows helper should read the authoritative workflow catalog');
  assertContains(workflowsSource, '/run', 'native workflows helper should run workflows through the authoritative run endpoint');
  assertContains(workflowsSource, '/runs', 'native workflows helper should load run history through the authoritative runs endpoint');
  assertContains(pageSource, 'await readWorkflows();', 'native workflows page should load the authoritative workflow library');
  assertContains(pageSource, 'await updateWorkflow(selected.id, payload)', 'native workflows page should update workflows through the helper');
  assertContains(pageSource, 'await createWorkflow(payload)', 'native workflows page should create workflows through the helper');
  assertContains(pageSource, 'await runWorkflow(selected.id, runInput)', 'native workflows page should run workflows through the helper');
  assertContains(pageSource, 'await readWorkflowRuns(selected.id)', 'native workflows page should load run history through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('workflows')}", 'native workflows should preserve a classic escape hatch');
  assertContains(editorSource, "dispatch('submit')", 'native workflow editor panel should expose saves via event dispatch');
  assertContains(editorSource, "dispatch('addstep')", 'native workflow editor panel should expose step creation via event dispatch');
  assertContains(runSource, "dispatch('run')", 'native workflow run panel should expose workflow execution via event dispatch');
  assertContains(routeSource, '<WorkflowsPage />', 'native workflows route should render the Svelte workflows page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native workflows route should keep static prerender options in +page.ts');
}

function assertNativeSessionsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const sessionsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/sessions.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/SessionsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/sessions/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/sessions/+page.ts'));

  assert.ok(/\{\s*key:\s*'sessions'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark sessions as a native route');
  assertContains(sessionsSource, '/api/sessions', 'native sessions helper should read the authoritative session inventory');
  assertContains(sessionsSource, '/api/memory/agents/${encodeURIComponent(agentId)}/kv', 'native sessions helper should read authoritative agent memory kv pairs');
  assertContains(sessionsSource, '/api/memory/agents/${encodeURIComponent(agentId)}/kv/${encodeURIComponent(key)}', 'native sessions helper should mutate authoritative memory kv pairs');
  assertContains(pageSource, 'await readSessions();', 'native sessions page should load session inventory through the helper');
  assertContains(pageSource, 'await readAgentMemoryKv(selectedAgentId);', 'native sessions page should load agent memory through the helper');
  assertContains(pageSource, 'await upsertAgentMemoryKv(selectedAgentId, keyDraft.trim(), value);', 'native sessions page should save memory through the helper');
  assertContains(pageSource, 'await deleteAgentMemoryKv(selectedAgentId, key);', 'native sessions page should delete memory through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('sessions')}", 'native sessions should preserve a classic escape hatch');
  assertContains(routeSource, '<SessionsPage />', 'native sessions route should render the Svelte sessions page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native sessions route should keep static prerender options in +page.ts');
}

function assertNativeSchedulerRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const schedulerSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/scheduler.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/SchedulerPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/scheduler/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/scheduler/+page.ts'));

  assert.ok(/\{\s*key:\s*'scheduler'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark scheduler as a native route');
  assertContains(schedulerSource, '/api/cron/jobs', 'native scheduler helper should read the authoritative cron jobs lane');
  assertContains(schedulerSource, '/api/triggers', 'native scheduler helper should read the authoritative trigger lane');
  assertContains(schedulerSource, '/api/schedules/${encodeURIComponent(jobId)}/run', 'native scheduler helper should run schedules through the authoritative schedule endpoint');
  assertContains(pageSource, 'await createCronJob({ agent_id: jobAgentId, name: jobName.trim(), cron: jobCron.trim(), message: jobMessage.trim(), enabled: jobEnabled });', 'native scheduler page should create schedules through the helper');
  assertContains(pageSource, 'setCronJobEnabled(job.id, !job.enabled)', 'native scheduler page should toggle schedule state through the helper');
  assertContains(pageSource, 'runCronJobNow(job.id)', 'native scheduler page should run schedules immediately through the helper');
  assertContains(pageSource, 'deleteTrigger(trigger.id)', 'native scheduler page should delete triggers through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('scheduler')}", 'native scheduler should preserve a classic escape hatch');
  assertContains(routeSource, '<SchedulerPage />', 'native scheduler route should render the Svelte scheduler page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native scheduler route should keep static prerender options in +page.ts');
}

function assertNativeEyesRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const eyesSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/eyes.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/EyesPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/eyes/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/eyes/+page.ts'));

  assert.ok(/\{\s*key:\s*'eyes'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark eyes as a native route');
  assertContains(eyesSource, '/api/eyes', 'native eyes helper should read the authoritative eye catalog');
  assertContains(eyesSource, "requestJson<JsonRecord>('POST', '/api/eyes', input)", 'native eyes helper should save eye configuration through the authoritative endpoint');
  assertContains(pageSource, 'await readEyes();', 'native eyes page should load the eye catalog through the helper');
  assertContains(pageSource, "await saveEye({ name, status, url, api_key: apiKey, cadence_hours: cadenceHours, topics });", 'native eyes page should save eyes through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('eyes')}", 'native eyes should preserve a classic escape hatch');
  assertContains(routeSource, '<EyesPage />', 'native eyes route should render the Svelte eyes page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native eyes route should keep static prerender options in +page.ts');
}

function assertNativeCommsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const commsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/comms.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/CommsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/comms/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/comms/+page.ts'));

  assert.ok(/\{\s*key:\s*'comms'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark comms as a native route');
  assertContains(commsSource, '/api/comms/topology', 'native comms helper should read the authoritative comms topology');
  assertContains(commsSource, '/api/comms/events?limit=200', 'native comms helper should read recent comms events');
  assertContains(commsSource, '/api/comms/send', 'native comms helper should send messages through the authoritative comms endpoint');
  assertContains(commsSource, '/api/comms/task', 'native comms helper should post tasks through the authoritative comms endpoint');
  assertContains(pageSource, 'setInterval(() => void refresh(), 5000)', 'native comms page should keep bounded polling refresh for the live comms slice');
  assertContains(pageSource, "await sendCommsMessage({ from_agent_id: sendFrom, to_agent_id: sendTo, message: sendMessage.trim() });", 'native comms page should send messages through the helper');
  assertContains(pageSource, "await postCommsTask({ title: taskTitle.trim(), description: taskDesc.trim(), assigned_to: taskAssign });", 'native comms page should post tasks through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('comms')}", 'native comms should preserve a classic escape hatch');
  assertContains(routeSource, '<CommsPage />', 'native comms route should render the Svelte comms page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native comms route should keep static prerender options in +page.ts');
}

function assertNativeChannelsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const channelsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/channels.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/ChannelsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/channels/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/channels/+page.ts'));

  assert.ok(/\{\s*key:\s*'channels'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark channels as a native route');
  assertContains(channelsSource, '/api/channels', 'native channels helper should read the authoritative channel catalog');
  assertContains(channelsSource, '/api/channels/${encodeURIComponent(name)}/configure', 'native channels helper should configure channels through the authoritative endpoint');
  assertContains(channelsSource, '/api/channels/${encodeURIComponent(name)}/test', 'native channels helper should test channels through the authoritative endpoint');
  assertContains(channelsSource, '/api/channels/whatsapp/qr/start', 'native channels helper should start WhatsApp QR flows through the authoritative endpoint');
  assertContains(channelsSource, '/api/channels/whatsapp/qr/status?session_id=', 'native channels helper should read WhatsApp QR status');
  assertContains(pageSource, 'await readChannels();', 'native channels page should load the channel catalog through the helper');
  assertContains(pageSource, 'await configureChannel(selectedChannel.name, fieldsPayload());', 'native channels page should save channel config through the helper');
  assertContains(pageSource, 'await testChannel(selectedChannel.name);', 'native channels page should test channels through the helper');
  assertContains(pageSource, 'await startWhatsappQr();', 'native channels page should support WhatsApp QR start through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('channels')}", 'native channels should preserve a classic escape hatch');
  assertContains(routeSource, '<ChannelsPage />', 'native channels route should render the Svelte channels page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native channels route should keep static prerender options in +page.ts');
}

function assertNativeSkillsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const skillsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/skills.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/SkillsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/skills/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/skills/+page.ts'));

  assert.ok(/\{\s*key:\s*'skills'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark skills as a native route');
  assertContains(skillsSource, '/api/skills', 'native skills helper should read installed skills through the authoritative endpoint');
  assertContains(skillsSource, '/api/mcp/servers', 'native skills helper should read MCP server status');
  assertContains(skillsSource, '/api/clawhub/browse', 'native skills helper should browse marketplace results');
  assertContains(skillsSource, '/api/clawhub/search', 'native skills helper should search marketplace results');
  assertContains(skillsSource, '/api/clawhub/install', 'native skills helper should install marketplace skills');
  assertContains(skillsSource, '/api/skills/create', 'native skills helper should create prompt-only skills');
  assertContains(skillsSource, '/api/skills/uninstall', 'native skills helper should uninstall skills');
  assertContains(pageSource, 'await readInstalledSkills()', 'native skills page should load installed skills through the helper');
  assertContains(pageSource, 'await browseMarketplace(sort)', 'native skills page should browse marketplace results through the helper');
  assertContains(pageSource, 'await searchMarketplace(search.trim())', 'native skills page should search marketplace results through the helper');
  assertContains(pageSource, 'await installMarketplaceSkill(slug)', 'native skills page should install marketplace skills through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('skills')}", 'native skills should preserve a classic escape hatch');
  assertContains(routeSource, '<SkillsPage />', 'native skills route should render the Svelte skills page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native skills route should keep static prerender options in +page.ts');
}

function assertNativeHandsRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const handsSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/hands.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/HandsPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/hands/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/hands/+page.ts'));

  assert.ok(/\{\s*key:\s*'hands'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark hands as a native route');
  assertContains(handsSource, '/api/hands', 'native hands helper should read the authoritative hands catalog');
  assertContains(handsSource, '/api/hands/active', 'native hands helper should read active hand instances');
  assertContains(handsSource, '/api/hands/${encodeURIComponent(handId)}/activate', 'native hands helper should activate hands through the authoritative endpoint');
  assertContains(handsSource, '/api/hands/instances/${encodeURIComponent(instanceId)}/pause', 'native hands helper should pause instances through the authoritative endpoint');
  assertContains(handsSource, '/api/hands/instances/${encodeURIComponent(instanceId)}/resume', 'native hands helper should resume instances through the authoritative endpoint');
  assertContains(pageSource, 'await readHandsCatalog()', 'native hands page should load the hands catalog through the helper');
  assertContains(pageSource, 'await readActiveHands()', 'native hands page should load active instances through the helper');
  assertContains(pageSource, 'await activateHand(selectedId, parsedConfig)', 'native hands page should activate hands through the helper');
  assertContains(pageSource, 'await checkHandDependencies(selectedId)', 'native hands page should support dependency checks through the helper');
  assertContains(pageSource, "href={dashboardClassicHref('hands')}", 'native hands should preserve a classic escape hatch');
  assertContains(routeSource, '<HandsPage />', 'native hands route should render the Svelte hands page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native hands route should keep static prerender options in +page.ts');
}

function assertNativeWizardRouteContract() {
  const dashboardSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/dashboard.ts'));
  const pageSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/lib/components/WizardPage.svelte'));
  const routeSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/wizard/+page.svelte'));
  const routeLoadSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/dashboard_sveltekit/src/routes/wizard/+page.ts'));

  assert.ok(/\{\s*key:\s*'wizard'[\s\S]{0,220}mode:\s*'native'/.test(dashboardSource), 'dashboard registry should mark wizard as a native route');
  assertContains(pageSource, 'await Promise.all([readProviders(), readTemplates(), readChannels()])', 'native wizard should load providers, templates, and channels through authoritative helpers');
  assertContains(pageSource, 'await saveProviderKey(selectedProvider.id, providerKey.trim())', 'native wizard should save provider keys through the authoritative settings helper');
  assertContains(pageSource, 'await testProvider(selectedProvider.id)', 'native wizard should test providers through the authoritative settings helper');
  assertContains(pageSource, 'await spawnTemplateAgent(selectedTemplate.name)', 'native wizard should create agents through the authoritative template flow');
  assertContains(pageSource, 'await createDraftAgent()', 'native wizard should keep a draft-agent fallback for empty template catalogs');
  assertContains(pageSource, 'await configureChannel(selectedChannel.name, fields)', 'native wizard should configure optional channels through the authoritative channels helper');
  assertContains(pageSource, 'await startWhatsappQr()', 'native wizard should support QR-based WhatsApp setup through the authoritative channels helper');
  assertContains(pageSource, "href={dashboardClassicHref('wizard')}", 'native wizard should preserve a classic escape hatch');
  assertContains(routeSource, '<WizardPage />', 'native wizard route should render the Svelte wizard page directly');
  assertContains(routeLoadSource, 'export const prerender = true;', 'native wizard route should keep static prerender options in +page.ts');
}

const runSnapshotAssertionsWithNativeChat = runSnapshotAssertions;
runSnapshotAssertions = function() {
  assertNativeChatRouteContract();
  assertNativeAgentsRouteContract();
  assertNativeSettingsRouteContract();
  assertNativeRuntimeRouteContract();
  assertNativeApprovalsRouteContract();
  assertNativeAnalyticsRouteContract();
  assertNativeLogsRouteContract();
  assertNativeWorkflowsRouteContract();
  assertNativeSessionsRouteContract();
  assertNativeSchedulerRouteContract();
  assertNativeEyesRouteContract();
  assertNativeCommsRouteContract();
  assertNativeChannelsRouteContract();
  assertNativeSkillsRouteContract();
  assertNativeHandsRouteContract();
  assertNativeWizardRouteContract();
  return runSnapshotAssertionsWithNativeChat();
};

const contract = getFlag('--contract');
const parseOnly = getFlag('--dashboard-inline-parse-only');
if (parseOnly) {
  assertDashboardInlineScriptsParse();
  assertDashboardBuildVersionFresh();
  assertDashboardVersionRefreshUsesApiVersion();
  assertTopbarHeroSystemMenu();
  assertDashboardHostOverlaysLiveVersion();
} else if (contract) {
  runContract(contract);
} else {
  runSnapshotAssertions();
}
