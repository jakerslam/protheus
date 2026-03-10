#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const SCRIPT = path.join(ROOT, '..', 'scripts', 'metrics', 'heartbeat_kpi_report.js');

try {
  const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'heartbeat-kpi-test-'));
  const outPath = path.join(tmp, 'latest.json');
  const historyPath = path.join(tmp, 'history.jsonl');
  const run = spawnSync(process.execPath, [SCRIPT], {
    cwd: path.join(ROOT, '..'),
    encoding: 'utf8',
    env: {
      ...process.env,
      PROTHEUS_HEARTBEAT_KPI_SKIP_DEPLOY: '1',
      PROTHEUS_HEARTBEAT_KPI_OUT: outPath,
      PROTHEUS_HEARTBEAT_KPI_HISTORY: historyPath
    }
  });

  assert.strictEqual(run.status, 0, run.stderr || run.stdout);
  assert.ok(fs.existsSync(outPath), 'latest report should be written');
  assert.ok(fs.existsSync(historyPath), 'history report should be appended');

  const latest = JSON.parse(fs.readFileSync(outPath, 'utf8'));
  assert.strictEqual(latest.type, 'heartbeat_kpi_report');
  assert.ok(latest.kpi && Number.isFinite(Number(latest.kpi.completion_rate)));
  assert.ok(latest.checks && latest.checks.deployment_health && latest.checks.deployment_health.skipped === true);

  const lines = fs.readFileSync(historyPath, 'utf8').trim().split('\n').filter(Boolean);
  assert.ok(lines.length >= 1, 'history should have at least one row');

  fs.rmSync(tmp, { recursive: true, force: true });
  console.log('heartbeat_kpi_report.test.js: OK');
} catch (err) {
  console.error(`heartbeat_kpi_report.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
