#!/usr/bin/env node
'use strict';

// Compatibility shim for older protheus-ops releases that still route setup
// to protheus_setup_wizard.js. Delegates to the TS lane when available.

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const TS_ENTRY = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const TS_TARGET = path.join(__dirname, 'protheus_setup_wizard.ts');
const STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'protheus_setup_wizard',
  'latest.json'
);

function nowIso() {
  return new Date().toISOString();
}

function writeFallbackState() {
  const payload = {
    type: 'protheus_setup_wizard_state',
    completed: true,
    completed_at: nowIso(),
    completion_mode: 'js_compat_fallback',
    node_runtime_detected: true,
    interaction_style: 'silent',
    notifications: 'none',
    covenant_acknowledged: false,
    version: 1
  };
  fs.mkdirSync(path.dirname(STATE_PATH), { recursive: true });
  fs.writeFileSync(STATE_PATH, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  return payload;
}

function runTsWizard() {
  const proc = spawnSync(
    process.execPath,
    [TS_ENTRY, TS_TARGET, ...process.argv.slice(2)],
    { stdio: 'inherit', cwd: ROOT }
  );
  process.exit(Number.isFinite(proc.status) ? proc.status : 1);
}

function main() {
  if (fs.existsSync(TS_ENTRY) && fs.existsSync(TS_TARGET)) {
    runTsWizard();
    return;
  }

  const jsonMode = process.argv.slice(2).some((arg) => {
    const token = String(arg || '').trim().toLowerCase();
    return token === '--json' || token === '--json=1';
  });
  const state = writeFallbackState();
  const payload = {
    ok: true,
    type: 'protheus_setup_wizard_fallback',
    mode: 'js_compat_fallback',
    state_path: STATE_PATH,
    state
  };
  if (jsonMode) {
    process.stdout.write(`${JSON.stringify(payload)}\n`);
    return;
  }
  process.stdout.write('[infring setup] compatibility fallback completed\n');
}

if (require.main === module) {
  main();
}
