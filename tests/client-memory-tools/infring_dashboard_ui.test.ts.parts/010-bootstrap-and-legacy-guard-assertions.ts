'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const TARGET_SOURCE = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const REMOVED_DASHBOARD_CLIENT_REL = [
  'client',
  'runtime',
  'systems',
  'ui',
  ['infring', 'dashboard', 'client.tsx'].join('_'),
].join('/');
const REMOVED_DASHBOARD_CSS_REL = [
  'client',
  'runtime',
  'systems',
  'ui',
  ['infring', 'dashboard.css'].join('_'),
].join('/');
const REMOVED_DASHBOARD_FAMILY = ['V6', 'DASHBOARD', '006'].join('-');
const REMOVED_SPEC_TOKEN = ['INFRING', 'DASHBOARD', 'UI', 'SPEC'].join('_');
const REMOVED_NODE_UI_FLAG = ['--node', 'ui'].join('-');
const REMOVED_LEGACY_NODE_UI_FLAG = ['--legacy', 'node', 'ui'].join('-');
const REMOVED_TOGGLE_CONTROLS_ACTION = ['dashboard.ui', 'toggleControls'].join('.');
const REMOVED_TOGGLE_SECTION_ACTION = ['dashboard.ui', 'toggleSection'].join('.');
const REMOVED_SWITCH_TAB_ACTION = ['dashboard.ui', 'switchControlsTab'].join('.');
const REMOVED_BROWSER_SHELL_TYPE = ['infring_dashboard_browser_shell', 'removed'].join('_');
const LEGACY_DASHBOARD_ARTIFACTS = [
  path.resolve(ROOT, REMOVED_DASHBOARD_CLIENT_REL),
  path.resolve(ROOT, `${REMOVED_DASHBOARD_CLIENT_REL}.parts`),
  path.resolve(ROOT, REMOVED_DASHBOARD_CSS_REL),
  path.resolve(ROOT, `${REMOVED_DASHBOARD_CSS_REL}.parts`),
  path.resolve(ROOT, 'docs/workspace', `${REMOVED_SPEC_TOKEN}.md`),
  path.resolve(ROOT, 'docs/workspace/DASHBOARD_AUTHORITY_PARITY_CHECKLIST.md'),
];
const CHAT_PAGE_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/pages/chat.ts'
);
const APP_STATIC_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/app.ts'
);
const API_STATIC_TS_PATH = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js/api.ts'
);
const STATIC_UI_JS_ROOT = path.resolve(
  ROOT,
  'client/runtime/systems/ui/infring_static/js'
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
    maxBuffer: 128 * 1024 * 1024,
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
  const ext = path.extname(filePath).toLowerCase();
  const partDirs = [`${filePath}.parts`];
  if (ext === '.js') {
    partDirs.push(filePath.replace(/\.js$/i, '.ts') + '.parts');
  } else if (ext === '.ts') {
    partDirs.push(filePath.replace(/\.ts$/i, '.js') + '.parts');
  }
  for (const partsDir of partDirs) {
    if (fs.existsSync(partsDir) && fs.statSync(partsDir).isDirectory()) {
      const partFiles = fs
        .readdirSync(partsDir, { withFileTypes: true })
        .filter((entry) => entry && entry.isFile())
        .map((entry) => entry.name)
        .filter((name) => path.extname(name).toLowerCase() === ext)
        .sort((a, b) => a.localeCompare(b, 'en'))
        .map((name) => fs.readFileSync(path.join(partsDir, name), 'utf8'));
      if (partFiles.length > 0) return partFiles.join('\n');
    }
  }
  return fs.readFileSync(filePath, 'utf8');
}

function isRustDashboardLaneWrapperSource(source) {
  const text = String(source || '');
  if (!text) return false;
  const legacyWrapperSignature =
    text.includes('Thin client wrapper only: delegates all dashboard authority to Rust core.') &&
    text.includes("runProtheusOps(['dashboard-ui'");
  const apiHostWrapperSignature =
    text.includes('Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.') &&
    text.includes("authority: 'primary_dashboard_ui_over_rust_core_api'") &&
    text.includes('function proxyToBackend(req, res, flags)');
  return legacyWrapperSignature || apiHostWrapperSignature;
}

function assertDashboardFileSizeCaps() {
  const uiRoot = path.resolve(ROOT, 'client/runtime/systems/ui');
  const sourceExts = new Set(['.ts', '.tsx', '.js', '.jsx', '.css', '.html']);
  const generatedDirs = new Set(['.svelte-kit', 'build', 'dist', 'node_modules']);
  const violations = [];
  const walk = (dir) => {
    if (!fs.existsSync(dir)) return;
    const entries = fs.readdirSync(dir, { withFileTypes: true });
    for (const entry of entries) {
      const fullPath = path.join(dir, entry.name);
      if (entry.isDirectory()) {
        if (generatedDirs.has(entry.name)) continue;
        walk(fullPath);
        continue;
      }
      const ext = path.extname(entry.name).toLowerCase();
      if (!sourceExts.has(ext)) continue;
      const vendorToken = `${path.sep}vendor${path.sep}`;
      if (fullPath.includes(vendorToken) || path.basename(path.dirname(fullPath)) === 'vendor') continue;
      const lines = fs.readFileSync(fullPath, 'utf8').split(/\r?\n/).length;
      if (lines <= 500) continue;
      const header = fs.readFileSync(fullPath, 'utf8').split(/\r?\n/).slice(0, 8).join('\n');
      if (/FILE_SIZE_EXCEPTION:\s*reason=.+owner=.+expires=\d{4}-\d{2}-\d{2}/.test(header)) continue;
      violations.push({
        file: path.relative(ROOT, fullPath),
        lines,
      });
    }
  };
  walk(uiRoot);
  assert.strictEqual(
    violations.length,
    0,
    `dashboard source files must stay <=500 LoC unless FILE_SIZE_EXCEPTION is declared: ${JSON.stringify(violations.slice(0, 12))}`
  );
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
  if (isRustDashboardLaneWrapperSource(haystack)) {
    // The dashboard lane authority moved into Rust core; string-level JS lane probes
    // are not valid in wrapper mode.
    return;
  }
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

function assertLegacyDashboardArtifactsRemoved() {
  for (const artifactPath of LEGACY_DASHBOARD_ARTIFACTS) {
    assert.ok(
      !fs.existsSync(artifactPath),
      `removed dashboard artifact should stay deleted: ${path.relative(ROOT, artifactPath)}`
    );
  }

  const readmeSource = readUtf8(path.resolve(ROOT, 'README.md'));
  const todoSource = readUtf8(path.resolve(ROOT, 'docs/workspace/TODO.md'));
  const srsSource = readUtf8(path.resolve(ROOT, 'docs/workspace/SRS.md'));
  const packageSource = readUtf8(path.resolve(ROOT, 'package.json'));
  const cliSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/protheusctl.rs'));
  const rustUiSource = readUtf8(path.resolve(ROOT, 'core/layer0/ops/src/dashboard_ui.rs'));
  const laneSource = readUtf8(TARGET_SOURCE);
  const cohesionSource = readUtf8(
    path.resolve(ROOT, 'client/runtime/config/module_cohesion_legacy_baseline.json')
  );

  assert.ok(!readmeSource.includes(REMOVED_NODE_UI_FLAG), 'README should not mention removed dashboard override flags');
  assert.ok(!todoSource.includes(REMOVED_DASHBOARD_FAMILY), 'TODO should not track the removed compact dashboard family');
  assert.ok(!srsSource.includes(REMOVED_DASHBOARD_FAMILY), 'SRS should not describe the removed compact dashboard family');
  assert.ok(!srsSource.includes(REMOVED_SPEC_TOKEN), 'SRS should not reference the removed dashboard spec');
  assert.ok(!packageSource.includes(REMOVED_DASHBOARD_FAMILY), 'package.json should not expose removed dashboard contract lanes');
  assert.ok(!cliSource.includes(REMOVED_NODE_UI_FLAG), 'CLI should not accept removed dashboard override flags');
  assert.ok(!cliSource.includes(REMOVED_LEGACY_NODE_UI_FLAG), 'CLI should not accept removed dashboard fallback flags');
  assert.ok(!laneSource.includes(REMOVED_TOGGLE_CONTROLS_ACTION), 'dashboard host should not retain removed compact-dashboard actions');
  assert.ok(!laneSource.includes(REMOVED_TOGGLE_SECTION_ACTION), 'dashboard host should not retain removed compact-dashboard section actions');
  assert.ok(!laneSource.includes(REMOVED_SWITCH_TAB_ACTION), 'dashboard host should not retain removed compact-dashboard tab actions');
  assert.ok(!rustUiSource.includes(REMOVED_BROWSER_SHELL_TYPE), 'Rust API lane should not advertise the removed legacy shell');
  assert.ok(
    !cohesionSource.includes(REMOVED_DASHBOARD_CLIENT_REL),
    'module cohesion baseline should not retain deleted dashboard client paths'
  );
}

function assertChatSyntaxGuards() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  assert.ok(
    !/async\s+resolveArtifactDirectives\s*:\s*function/.test(chatSource),
    'invalid async object-property syntax in chat page can break dashboard script boot'
  );
  assert.ok(
    /resolveArtifactDirectives\s*:\s*async\s+function(?:\s+\w+)?\s*\(/.test(chatSource),
    'resolveArtifactDirectives must be declared as async function property'
  );
}

function assertChatEnhancementFeatures() {
  const chatSource = readUtf8(CHAT_PAGE_TS_PATH);
  const apiSource = readUtf8(API_STATIC_TS_PATH);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const cssSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/css/components.css'));
  const laneSource = readUtf8(TARGET_SOURCE);

  // Fresh agent init flow ("Who am I?" + init panel)
  assertContains(chatSource, "text: 'Who am I?'", 'fresh-init "Who am I?" seed message missing');
  assert.ok(
    chatSource.includes('ensureFreshInitThread(resolved);') || chatSource.includes('startFreshInitSequence(resolved);'),
    'fresh-init thread bootstrap missing'
  );
  assertContains(htmlSource, 'class="chat-init-panel"', 'fresh-init panel markup missing');
  assertContains(htmlSource, 'Initialize Agent', 'fresh-init panel title missing');
  assertContains(htmlSource, 'Advanced setup', 'fresh-init advanced setup toggle missing');
  assertContains(htmlSource, 'chat-init-model-grid', 'fresh-init LLM suggestion grid missing');
  assertContains(htmlSource, 'Vibe', 'fresh-init vibe section missing');
  assertContains(chatSource, 'refreshFreshInitModelSuggestions: async function(templateDef)', 'fresh-init role-based LLM ranking helper missing');
  assertContains(chatSource, 'scoreFreshInitModelForRole: function(model, roleKey)', 'fresh-init model scoring function missing');
  assertContains(chatSource, 'freshInitModelSelection = ranked.length ? this.normalizeFreshInitModelRef(ranked[0]) : \'\';', 'fresh-init should auto-select top-ranked model by default');
  assertContains(cssSource, '.chat-init-advanced-toggle', 'fresh-init advanced toggle styles missing');
  assertContains(cssSource, '.chat-init-model-meta', 'fresh-init model metadata row styles missing');
  assertContains(chatSource, 'sessionHasAnyHistory: function(data)', 'empty-session history detector missing');
  assertContains(chatSource, 'recoverEmptySessionRender: function(agentId, sessionPayload)', 'empty-session render recovery helper missing');
  assertContains(chatSource, 'pinToLatestOnOpen: function(container, options)', 'chat open pin-to-latest helper missing');
  assertContains(chatSource, 'cancelPinToLatestOnOpen: function()', 'chat open pin cancel helper missing');
  assertContains(chatSource, 'self.pinToLatestOnOpen(null, { maxFrames: 24 });', 'session loader should re-pin to latest after render settles');
  assertContains(chatSource, 'scrollBottomBufferPx: 84', 'chat bottom buffer baseline should preserve visual padding without blank over-scroll');
  assertContains(chatSource, 'scrollBottomClampSlackPx: 16', 'chat bottom clamp slack tuning missing');
  assertContains(chatSource, 'page && page.showFreshArchetypeTiles', 'fresh-init mode should bypass hard bottom clamp');
  assertContains(chatSource, 'setTimeout(function() { host.scrollTop = Math.min(Number(host.scrollTop || 0), resolveLatestMessageScrollTop(page, host));', 'bottom clamp should defer correction to avoid scroll thrash');
  assertContains(chatSource, "text: 'This session is empty. Send a message to begin.'", 'empty-session fallback message missing');
  assertContains(chatSource, 'self.recoverEmptySessionRender(agentId, data || null);', 'empty-session recovery hook missing from session loader');
  assertContains(
    htmlSource,
    "x-if=\"currentAgent && !sessionLoading && (!messages || messages.length === 0) && !showFreshArchetypeTiles\"",
    'primary chat empty-session fallback UI missing'
  );
  assertContains(
    htmlSource,
    "x-if=\"currentAgent && !sessionLoading && (!filteredMessages || filteredMessages.length === 0) && !showFreshArchetypeTiles\"",
    'inline filtered chat empty-session fallback UI missing'
  );

  // Prompt suggestion chips above composer
  assertContains(chatSource, 'refreshPromptSuggestions', 'prompt suggestion refresh flow missing');
  assertContains(chatSource, "/api/agents/' + encodeURIComponent(agentId) + '/suggestions", 'suggestion API client call missing');
  assertContains(chatSource, 'collectPromptSuggestionContext()', 'prompt suggestion context extractor missing');
  assertContains(chatSource, 'payload.recent_context = String(context.signature).trim();', 'prompt suggestion request should include recent context signature');
  assertContains(chatSource, '/^(post-(response|silent|error|terminal)|init|refresh)$/i.test(cleanHint)', 'prompt suggestion hint sanitizer missing in chat client');
  assertContains(chatSource, 'row = clampWords(row, 10);', 'suggestion normalizer should preserve full 10-word budget');
  assertContains(chatSource, 'if (words < 3 || words > 10) return true;', 'suggestion normalizer word budget guard missing');
  assertContains(chatSource, "rows.push('Tell me more about ' + seed);", 'template-driven suggestion fallback missing');
  assertContains(chatSource, "rows.push('What are next steps for ' + seed);", 'next-step suggestion template missing');
  assertContains(chatSource, "rows.push('Can you verify ' + seed + ' works');", 'verification suggestion template missing');
  assertContains(laneSource, 'Do not echo instructions or policy text.', 'prompt suggestion generator anti-echo guard missing');
  assertContains(laneSource, 'META_SUGGESTION_PATTERNS', 'prompt suggestion meta-pattern scrubber missing');
  assertContains(laneSource, 'function sanitizeSuggestionHint(value)', 'dashboard suggestion hint sanitizer missing');
  assert.ok(
    !laneSource.includes('highest-ROI fix for this task'),
    'prompt suggestion fallback should not emit generic "this task" phrasing'
  );
  assertContains(htmlSource, 'class="prompt-suggestions-row"', 'prompt suggestion row missing');
  assertContains(htmlSource, 'prompt-suggestion-chip', 'prompt suggestion chip missing');
  assertContains(chatSource, 'appendUserChatMessage: function(finalText, msgImages, options)', 'queued prompt render helper missing');
  assertContains(chatSource, 'this.appendUserChatMessage(nextText, nextImages, { deferPersist: true });', 'queued prompts must render only when dequeued');
  assertContains(chatSource, "this.appendUserChatMessage(finalText, msgImages, { deferPersist: true });", 'immediate dispatch should render via shared append helper');
  assertContains(chatSource, 'queue_id: next && next.queue_id ? String(next.queue_id) : \'\'', 'dequeue payload should keep queue id context');
  assertContains(cssSource, 'max-width: 90%;', 'prompt queue stack should cap width at 90%');
  assertContains(cssSource, '.prompt-queue-item:first-child', 'prompt queue top item selector missing');
  assertContains(cssSource, 'border-top-left-radius: 10px;', 'prompt queue top item top-left radius missing');
  assertContains(cssSource, 'border-top-right-radius: 10px;', 'prompt queue top item top-right radius missing');

  // Local model download flow in model switcher
  assertContains(chatSource, 'downloadModelToLocal: function(model)', 'model download action handler missing');
  assertContains(chatSource, "InfringAPI.post('/api/models/download'", 'model download API call missing');
  assertContains(chatSource, 'isModelDownloadable: function(model)', 'model download availability helper missing');
  assertContains(chatSource, 'modelSpecialtyLabel: function(model)', 'model specialty label helper missing');
  assertContains(htmlSource, 'class="model-download-inline-btn"', 'model download button missing in model switcher');
  assertContains(htmlSource, 'class="model-meta-stat model-meta-specialty"', 'model specialty metadata row missing');
  assertContains(laneSource, "req.method === 'POST' && pathname === '/api/models/download'", 'model download backend endpoint missing');
  assertContains(laneSource, 'function inferSystemSpecProfile()', 'local model recommendation should derive system profile');
  assertContains(laneSource, 'function maybeEmitLocalModelBootstrapReminder(snapshot, options = {})', 'startup local-model reminder helper missing');
  assertContains(laneSource, 'Download or connect a local LLM to enable offline mode.', 'offline local-model startup notice text missing');
  assertContains(laneSource, 'function assignSubagentModelOverride(agentId, snapshot, options = {})', 'subagent model routing helper missing');

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
  assertContains(apiSource, 'upload_endpoint_stub_requires_dashboard_restart', 'upload client should detect stale compat-stub responses');
  assertContains(chatSource, "Failed to upload ' + att.file.name + ': ' + reason", 'upload failure toast should include backend reason');
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
  assertContains(chatSource, 'User: ... Agent: <answer>', 'chat transcript-leak sanitizer doc missing');
  assert.ok(
    chatSource.includes("var systemOrigin = m && m.system_origin ? String(m.system_origin) : '';") ||
      chatSource.includes('var derivedSystemOrigin = \'\';'),
    'system origin extraction missing'
  );
  assertContains(chatSource, 'system_origin: systemOrigin,', 'system origin normalization missing');
  assert.ok(
    chatSource.includes("return 'system:legacy:' + legacySystemId.toLowerCase();") ||
      chatSource.includes("return 'system';"),
    'system message source key grouping fallback missing'
  );
  assertContains(chatSource, "system_origin: 'slash:help'", 'slash help messages should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'agent:inactive'", 'inactive agent notices should carry explicit system origin');
  assertContains(chatSource, "system_origin: 'runtime:error'", 'runtime error messages should carry explicit system origin');
  assertContains(
    laneSource,
    'User: ... Agent: <final>',
    'server transcript-leak sanitizer doc missing'
  );
  assertContains(chatSource, '/^task accepted\\.\\s*report findings in this thread with receipt-backed evidence\\.?$/i.test(compactText)', 'legacy runtime task ack rows should be filtered from render history');
  assertContains(chatSource, 'return null;', 'legacy runtime task noise should be dropped during session normalization');
  assertContains(chatSource, '}).filter(function(row) { return !!row; });', 'session normalization should filter null rows');
  assertContains(chatSource, 'agent_id: data && data.agent_id ? String(data.agent_id)', 'live ws agent id propagation missing');
  assertContains(chatSource, 'agent_name: data && data.agent_name ? String(data.agent_name)', 'live ws agent name propagation missing');
}

function assertMemoryApiWired() {
  var laneSource = readUtf8(TARGET_SOURCE);
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

function assertEyesPageWired() {
  const laneSource = readUtf8(TARGET_SOURCE);
  const htmlSource = readUtf8(path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/index_body.html'));
  const appSource = readUtf8(APP_STATIC_TS_PATH);
  const eyesPagePath = path.resolve(ROOT, 'client/runtime/systems/ui/infring_static/js/pages/eyes.ts');
