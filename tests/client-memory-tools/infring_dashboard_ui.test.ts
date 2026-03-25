#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const CLIENT_TSX_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_dashboard_client.tsx'
);
const CLIENT_CSS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_dashboard.css'
);
const CHAT_PAGE_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/openclaw_static/js/pages/chat.ts'
);
const STATIC_UI_JS_ROOT = path.resolve(
  ROOT,
  'client/runtime/systems/ui/openclaw_static/js'
);
const SNAPSHOT_PATH = path.resolve(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json'
);
const STATIC_UI_AUTHORITY_PATTERNS = [
  /\brunLaneCached?\s*\(/,
  /\bspawnSync\s*\(/,
  /\bexecSync\s*\(/,
  /\bchild_process\b/,
  /\bprotheus-ops\b/,
  /\bcollab-plane\b/,
  /\battention-queue\b/,
  /\bhermes-plane\b/,
  /\bmodel-router\b/,
  /\bterminate-role\b/,
  /\bbacklog-delivery-plane\b/,
  /\bdashboard_runtime_authority\b/,
];

function runSnapshot() {
  return spawnSync(process.execPath, [ENTRYPOINT, TARGET, 'snapshot', '--pretty=0'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: 'pipe',
    env: process.env,
    maxBuffer: 16 * 1024 * 1024,
  });
}

function parseJson(text) {
  const raw = String(text || '').trim();
  assert(raw.length > 0, 'snapshot output should not be empty');
  return JSON.parse(raw);
}

function getFlag(name) {
  const prefix = `${name}=`;
  const row = process.argv.slice(2).find((entry) => String(entry).startsWith(prefix));
  if (!row) return '';
  return String(row).slice(prefix.length).trim();
}

function readUtf8(filePath) {
  return fs.readFileSync(filePath, 'utf8');
}

function walkUiJsFiles(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  const entries = fs.readdirSync(dir, { withFileTypes: true });
  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      walkUiJsFiles(fullPath, out);
      continue;
    }
    if (!/\.(js|ts)$/.test(entry.name)) continue;
    out.push(fullPath);
  }
  return out;
}

function assertContains(haystack, needle, message) {
  assert.ok(String(haystack).includes(needle), message || `missing: ${needle}`);
}

function assertThinClientAuthorityBoundary() {
  const files = walkUiJsFiles(STATIC_UI_JS_ROOT);
  const offenders = [];
  for (const filePath of files) {
    const source = readUtf8(filePath);
    for (const pattern of STATIC_UI_AUTHORITY_PATTERNS) {
      if (!pattern.test(source)) continue;
      offenders.push({
        file: path.relative(ROOT, filePath),
        marker: String(pattern),
      });
    }
  }
  assert.strictEqual(
    offenders.length,
    0,
    `browser UI must stay thin-client (no runtime authority primitives): ${JSON.stringify(offenders.slice(0, 10))}`
  );
}

function assertChatSyntaxGuards() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  assert.ok(
    !/async\s+resolveArtifactDirectives\s*:\s*function/.test(chatSource),
    'invalid async object-property syntax in chat page can break dashboard script boot'
  );
  assert.ok(
    /resolveArtifactDirectives\s*:\s*async\s+function/.test(chatSource),
    'resolveArtifactDirectives must be declared as async function property'
  );
}

function assertChatEnhancementFeatures() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/openclaw_static/index_body.html'));
  const cssSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/openclaw_static/css/components.css'));
  const laneSource = readUtf8(TARGET);

  // Fresh agent init flow ("Who am I?" + init panel)
  assertContains(chatSource, "text: 'Who am I?'", 'fresh-init "Who am I?" seed message missing');
  assertContains(chatSource, 'ensureFreshInitThread(resolved);', 'fresh-init thread bootstrap missing');
  assertContains(htmlSource, 'class="chat-init-panel"', 'fresh-init panel markup missing');
  assertContains(htmlSource, 'Initialize Agent', 'fresh-init panel title missing');

  // Prompt suggestion chips above composer
  assertContains(chatSource, 'refreshPromptSuggestions', 'prompt suggestion refresh flow missing');
  assertContains(chatSource, "/api/agents/' + encodeURIComponent(agentId) + '/suggestions", 'suggestion API client call missing');
  assertContains(chatSource, 'collectPromptSuggestionContext()', 'prompt suggestion context extractor missing');
  assertContains(chatSource, 'payload.recent_context = String(context.signature).trim();', 'prompt suggestion request should include recent context signature');
  assertContains(chatSource, '/^(post-(response|silent|error|terminal)|init|refresh)$/i.test(cleanHint)', 'prompt suggestion hint sanitizer missing in chat client');
  assertContains(laneSource, 'Do not echo instructions or policy text.', 'prompt suggestion generator anti-echo guard missing');
  assertContains(laneSource, 'META_SUGGESTION_PATTERNS', 'prompt suggestion meta-pattern scrubber missing');
  assertContains(laneSource, 'function sanitizeSuggestionHint(value)', 'dashboard suggestion hint sanitizer missing');
  assert.ok(
    !laneSource.includes('highest-ROI fix for this task'),
    'prompt suggestion fallback should not emit generic "this task" phrasing'
  );
  assertContains(htmlSource, 'class="prompt-suggestions-row"', 'prompt suggestion row missing');
  assertContains(htmlSource, 'class="prompt-suggestion-chip"', 'prompt suggestion chip missing');

  // Pointer effects: neon trail in dark mode + ripple in light mode
  assertContains(chatSource, 'handleMessagesPointerMove(event)', 'pointer move handler missing');
  assertContains(chatSource, 'handleMessagesPointerDown(event)', 'pointer down handler missing');
  assertContains(cssSource, '.chat-pointer-trail-dot', 'pointer trail style missing');
  assertContains(cssSource, '.chat-pointer-ripple', 'pointer ripple style missing');
  assertContains(cssSource, "body[data-theme='dark'] .chat-pointer-trail-dot", 'dark neon pointer style missing');

  // Artifact output: full file and folder tree + downloadable archive
  assertContains(chatSource, "case '/file':", 'slash command /file missing');
  assertContains(chatSource, "case '/folder':", 'slash command /folder missing');
  assertContains(laneSource, "parts[3] === 'file' && parts[4] === 'read'", 'lane file-read endpoint missing');
  assertContains(laneSource, "parts[3] === 'folder' && parts[4] === 'export'", 'lane folder-export endpoint missing');
  assertContains(laneSource, "pathname.startsWith('/api/chat/export/')", 'chat export download endpoint missing');
  assertContains(htmlSource, 'msg.file_output && msg.file_output.path', 'file output chat render missing');
  assertContains(htmlSource, 'msg.folder_output && msg.folder_output.path', 'folder output chat render missing');
  assertContains(htmlSource, 'class="chat-folder-download-link"', 'folder archive download link missing');

  // Progress UI (0-100%)
  assertContains(chatSource, 'parseProgressFromText', 'progress parser missing');
  assertContains(chatSource, 'messageProgress: function(msg)', 'progress accessor missing');
  assertContains(chatSource, 'progressFillStyle: function(msg)', 'progress style function missing');
  assertContains(htmlSource, 'class="chat-progress-wrap"', 'chat progress UI wrapper missing');
  assertContains(htmlSource, 'class="chat-progress-fill"', 'chat progress fill UI missing');

  // Multi-origin source-run grouping (group chat prep)
  assertContains(chatSource, 'messageSourceKey: function(msg)', 'message source-key resolver missing');
  assertContains(chatSource, 'isFirstInSourceRun: function(idx, rows)', 'first-in-run helper missing');
  assertContains(chatSource, 'isLastInSourceRun: function(idx, rows)', 'last-in-run helper missing');
  assertContains(chatSource, 'showMessageTitle(msg, idx, rows)', 'source-run title visibility helper missing');
  assertContains(
    htmlSource,
    "x-show=\"showMessageTitle(msg, idx, messages)\"",
    'primary message list missing source-run title wiring'
  );
  assertContains(
    htmlSource,
    "x-show=\"showMessageTitle(msg, idx, filteredMessages)\"",
    'filtered message list missing source-run title wiring'
  );
  assertContains(
    htmlSource,
    "isGrouped(idx, messages)",
    'primary message list missing source-run grouped wiring'
  );
  assertContains(
    htmlSource,
    "isGrouped(idx, filteredMessages)",
    'filtered message list missing source-run grouped wiring'
  );
  assertContains(cssSource, '.message.system.has-tail', 'system-tail render support missing');
  assertContains(chatSource, 'source_agent_id: m && m.source_agent_id ? String(m.source_agent_id) : \'\'', 'source agent id normalization missing');
  assertContains(chatSource, 'agent_origin: m && m.agent_origin ? String(m.agent_origin) : \'\'', 'agent origin normalization missing');
  assertContains(chatSource, "var systemOrigin = m && m.system_origin ? String(m.system_origin) : '';", 'system origin extraction missing');
  assertContains(chatSource, 'system_origin: systemOrigin,', 'system origin normalization missing');
  assertContains(chatSource, "return 'system:legacy:' + legacySystemId.toLowerCase();", 'legacy system message source key fallback missing');
  assertContains(chatSource, "system_origin: 'slash:help'", 'slash help messages should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'agent:inactive'", 'inactive agent notices should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'runtime:error'", 'runtime error messages should carry explicit system origin');
  assertContains(chatSource, '/^task accepted\\.\\s*report findings in this thread with receipt-backed evidence\\.?$/i.test(compactText)', 'legacy runtime task ack rows should be filtered from render history');
  assertContains(chatSource, 'return null;', 'legacy runtime task noise should be dropped during session normalization');
  assertContains(chatSource, '}).filter(function(row) { return !!row; });', 'session normalization should filter null rows');
  assertContains(chatSource, 'agent_id: data && data.agent_id ? String(data.agent_id)', 'live ws agent id propagation missing');
  assertContains(chatSource, 'agent_name: data && data.agent_name ? String(data.agent_name)', 'live ws agent name propagation missing');
}

function assertMemoryApiWired() {
  var laneSource = readUtf8(TARGET);
  assertContains(
    laneSource,
    "pathname.startsWith('/api/memory/agents/')",
    'memory kv API route missing (agent-scoped memory must not fall through compat stubs)'
  );
  assertContains(
    laneSource,
    "pathname === '/api/memory/search' || pathname === '/api/memory_search'",
    'memory search API fallback route missing'
  );
  assertContains(
    laneSource,
    'recordPassiveConversationMemory(agentId, userText, assistantText, metaText);',
    'passive memory ingestion hook missing from chat conversation append path'
  );
}

function assertAgentGitTreeAuthority() {
  const laneSource = readUtf8(TARGET);
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
}

function runSnapshotAssertions() {
  assertThinClientAuthorityBoundary();
  assertChatSyntaxGuards();
  assertChatEnhancementFeatures();
  assertMemoryApiWired();
  assertAgentGitTreeAuthority();
  const proc = runSnapshot();
  assert.strictEqual(proc.status, 0, `snapshot command failed: ${proc.stderr || proc.stdout}`);

  const payload = parseJson(proc.stdout);
  assert.strictEqual(payload.type, 'infring_dashboard_snapshot');
  assert.strictEqual(payload.metadata.authority, 'rust_core_lanes');
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

function assertContract0061() {
  const source = readUtf8(CLIENT_TSX_PATH);
  assertContains(source, "InfRing Chat", 'chat-first top bar title missing');
  assertContains(
    source,
    "Simple default chat. Open Controls only when needed.",
    'minimal-chat guidance missing'
  );
  assertContains(
    source,
    "const [controlsOpen, setControlsOpen] = useState<boolean>(() => readControlsOpen());",
    'controls-open state bootstrap missing'
  );
  assertContains(
    source,
    "No messages yet. Ask anything or type \"new agent\" to begin.",
    'empty-chat onboarding text missing'
  );
  assertContains(
    source,
    "placeholder=\"Ask anything or type 'new agent' to begin...\"",
    'chat placeholder missing'
  );
}

function assertContract0062() {
  const source = readUtf8(CLIENT_TSX_PATH);
  assertContains(source, "{ id: 'chat', label: 'Chat' }", 'chat pane missing');
  assertContains(
    source,
    "{ id: 'swarm', label: 'Swarm / Agent Management' }",
    'swarm pane missing'
  );
  assertContains(source, "{ id: 'health', label: 'Runtime Health' }", 'health pane missing');
  assertContains(
    source,
    "{ id: 'receipts', label: 'Receipts & Audit' }",
    'receipts pane missing'
  );
  assertContains(source, "{ id: 'logs', label: 'Logs' }", 'logs pane missing');
  assertContains(source, "{ id: 'settings', label: 'Settings' }", 'settings pane missing');
  assertContains(
    source,
    "await runAction('dashboard.ui.toggleControls', { open });",
    'controls toggle receipt route missing'
  );
  assertContains(
    source,
    "void runAction('dashboard.ui.toggleSection', { section: id, open: nextOpen });",
    'section toggle receipt route missing'
  );
  assertContains(
    source,
    "await runAction('dashboard.ui.switchControlsTab', { tab: 'swarm' });",
    'controls tab switch receipt route missing'
  );
  assertContains(
    source,
    "window.localStorage.setItem(CONTROLS_OPEN_KEY, controlsOpen ? '1' : '0');",
    'controls-open persistence missing'
  );
  assertContains(
    source,
    "window.localStorage.setItem(PANES_KEY, JSON.stringify(openPanes));",
    'pane-state persistence missing'
  );
}

function assertContract0063() {
  const source = readUtf8(CLIENT_TSX_PATH);
  assertContains(source, "'new_agent'", 'quick action kind new_agent missing');
  assertContains(source, "'new_swarm'", 'quick action kind new_swarm missing');
  assertContains(source, "'assimilate'", 'quick action kind assimilate missing');
  assertContains(source, "'benchmark'", 'quick action kind benchmark missing');
  assertContains(source, "'open_controls'", 'quick action kind open_controls missing');
  assertContains(source, "'swarm'", 'quick action kind swarm missing');
  assertContains(source, 'New Agent', 'New Agent quick chip missing');
  assertContains(source, 'New Swarm', 'New Swarm quick chip missing');
  assertContains(source, 'Assimilate Codex', 'Assimilate quick chip missing');
  assertContains(source, 'Run Benchmark', 'Run Benchmark quick chip missing');
  assertContains(source, 'Open Controls', 'Open Controls quick chip missing');
  assertContains(source, 'Swarm Tab', 'Swarm Tab quick chip missing');
}

function assertContract0064() {
  const source = readUtf8(CLIENT_TSX_PATH);
  const css = readUtf8(CLIENT_CSS_PATH);
  const laneSource = readUtf8(TARGET);
  assertContains(
    source,
    "aria-label=\"Toggle light or dark mode\"",
    'theme toggle a11y label missing'
  );
  assertContains(source, "if (metaOrCtrl && event.key.toLowerCase() === 'k')", 'Cmd/Ctrl+K shortcut missing');
  assertContains(source, "if (event.key === 'Escape' && controlsOpen)", 'Esc close shortcut missing');
  assertContains(css, "@media (max-width: 1023px)", 'mobile layout media query missing');
  assertContains(css, "@media (prefers-reduced-motion: reduce)", 'reduced-motion policy missing');
  assertContains(
    laneSource,
    "if (normalizedAction === 'dashboard.ui.toggleControls')",
    'toggleControls action lane missing'
  );
  assertContains(
    laneSource,
    "if (normalizedAction === 'dashboard.ui.toggleSection')",
    'toggleSection action lane missing'
  );
  assertContains(
    laneSource,
    "if (normalizedAction === 'dashboard.ui.switchControlsTab')",
    'switchControlsTab action lane missing'
  );
  assertContains(laneSource, 'function writeActionReceipt(action, payload, laneResult)', 'action receipt writer missing');
  assertContains(
    laneSource,
    'const actionReceipt = writeActionReceipt(action, actionPayload, laneResult);',
    'action receipt persistence call missing'
  );
}

function assertContract008() {
  const laneSource = readUtf8(TARGET);
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
  const laneSource = readUtf8(TARGET);
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
  const laneSource = readUtf8(TARGET);
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
  if (contract === 'V6-DASHBOARD-006.1') return assertContract0061();
  if (contract === 'V6-DASHBOARD-006.2') return assertContract0062();
  if (contract === 'V6-DASHBOARD-006.3') return assertContract0063();
  if (contract === 'V6-DASHBOARD-006.4') return assertContract0064();
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

const contract = getFlag('--contract');
if (contract) {
  runContract(contract);
} else {
  runSnapshotAssertions();
}
