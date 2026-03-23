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
const SNAPSHOT_PATH = path.resolve(
  ROOT,
  'client/runtime/local/state/ui/infring_dashboard/latest_snapshot.json'
);

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

function assertContains(haystack, needle, message) {
  assert.ok(String(haystack).includes(needle), message || `missing: ${needle}`);
}

function runSnapshotAssertions() {
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

function runContract(contract) {
  runSnapshotAssertions();
  if (contract === 'V6-DASHBOARD-006.1') return assertContract0061();
  if (contract === 'V6-DASHBOARD-006.2') return assertContract0062();
  if (contract === 'V6-DASHBOARD-006.3') return assertContract0063();
  if (contract === 'V6-DASHBOARD-006.4') return assertContract0064();
  assert.fail(`unsupported_contract:${contract}`);
}

const contract = getFlag('--contract');
if (contract) {
  runContract(contract);
} else {
  runSnapshotAssertions();
}
