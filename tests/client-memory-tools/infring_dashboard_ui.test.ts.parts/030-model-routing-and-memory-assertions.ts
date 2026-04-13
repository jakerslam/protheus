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
  assertContains(
    componentSource,
    "href={dashboardClassicHref('chat')}",
    'native chat should preserve a classic escape hatch while advanced legacy features remain'
  );
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

const runSnapshotAssertionsWithNativeChat = runSnapshotAssertions;
runSnapshotAssertions = function() {
  assertNativeChatRouteContract();
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
