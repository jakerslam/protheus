#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const RUNNER = path.join(ROOT, 'tests', 'tooling', 'scripts', 'ci', 'srs_repair_lane_runner.ts');
const SYSTEM_MAP = JSON.parse(fs.readFileSync(path.join(ROOT, 'client', 'runtime', 'config', 'system_map_registry.json'), 'utf8'));
const LANE_REGISTRY = JSON.parse(fs.readFileSync(path.join(ROOT, 'client', 'runtime', 'config', 'lane_command_registry.json'), 'utf8'));
const POLICY = JSON.parse(fs.readFileSync(path.join(ROOT, 'client', 'runtime', 'config', 'browser', 'browser_text_diff_policy.json'), 'utf8'));
const RECEIPT = path.join(ROOT, 'local', 'state', 'ops', 'srs_contract_runtime', 'V6-BROWSER-007', 'latest.json');

const systemEntry = (SYSTEM_MAP.entries || []).find((entry) => entry.id === 'browser_text_diff');
assert(systemEntry, 'missing browser_text_diff system map entry');
assert.equal(systemEntry.health_check, 'npm run -s test:lane:run -- --id=V6-BROWSER-007');
assert.equal(POLICY.event_stream.stream, 'browser.text_diff');
assert.equal(LANE_REGISTRY.test['V6-BROWSER-007'].source_script, 'test:lane:v6-browser-007');

const proc = spawnSync(process.execPath, [ENTRYPOINT, RUNNER, '--id=V6-BROWSER-007', '--dry-run=1'], {
  cwd: ROOT,
  encoding: 'utf8'
});
assert.equal(proc.status, 0, proc.stderr || proc.stdout);
const payload = JSON.parse(proc.stdout.trim());
assert.equal(payload.receiptPath, RECEIPT);

console.log(JSON.stringify({ ok: true, type: 'browser_text_diff_lane_test' }));
