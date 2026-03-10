#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const BRIDGE_PATH = [
  path.join(ROOT, 'runtime', 'systems', 'ops', 'reminder_data_bridge.js'),
  path.join(ROOT, 'systems', 'ops', 'reminder_data_bridge.js')
].find((candidate) => fs.existsSync(candidate));

if (!BRIDGE_PATH) {
  throw new Error('reminder_data_bridge_missing');
}

const bridge = require(BRIDGE_PATH);

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function withTempRoot(run) {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'reminder-bridge-'));
  try {
    return run(tempRoot);
  } finally {
    fs.rmSync(tempRoot, { recursive: true, force: true });
  }
}

try {
  withTempRoot((tempRoot) => {
    process.env.PROTHEUS_REMINDER_ROOT = tempRoot;
    const empty = bridge.buildSlackStatusSnapshot(tempRoot);
    assert.strictEqual(empty.ready, false, 'empty state should not be ready');
    assert.ok(Array.isArray(empty.missing) && empty.missing.length >= 3, 'empty snapshot should report missing signals');
    assert.strictEqual(empty.cadence_hours, 24, 'fallback cadence should be daily when state is incomplete');
  });

  withTempRoot((tempRoot) => {
    const stateRoot = path.join(tempRoot, 'client', 'runtime', 'local', 'state');
    writeJson(path.join(stateRoot, 'spine', 'runs', 'latest.json'), {
      ts: '2026-03-09T12:00:00.000Z',
      result: 'ok'
    });
    fs.mkdirSync(path.join(stateRoot, 'attention'), { recursive: true });
    fs.writeFileSync(path.join(stateRoot, 'attention', 'queue.jsonl'), '{"id":"q1"}\n{"id":"q2"}\n', 'utf8');
    writeJson(path.join(stateRoot, 'eye', 'latest.json'), {
      ts: '2026-03-09T12:01:00.000Z',
      status: 'ok'
    });
    writeJson(path.join(stateRoot, 'dopamine', 'ambient', 'latest.json'), {
      ts: '2026-03-09T12:02:00.000Z',
      summary: { sds: 11 }
    });
    writeJson(path.join(stateRoot, 'sensory', 'cross_signal', 'hypotheses', '2026-03-09.json'), {
      hypotheses: [{ id: 'h1' }]
    });

    process.env.PROTHEUS_REMINDER_ROOT = tempRoot;
    const full = bridge.buildSlackStatusSnapshot(tempRoot);
    assert.strictEqual(full.ready, true, 'snapshot should be ready when all core signals exist');
    assert.strictEqual(full.summary.proposal_queue_depth, 2, 'queue depth should count jsonl entries');
    assert.strictEqual(full.summary.dopamine.sds, 11, 'dopamine score should be extracted');
  });

  withTempRoot((tempRoot) => {
    process.env.PROTHEUS_REMINDER_ROOT = tempRoot;
    const noAccess = bridge.buildMoltcheckSnapshot(tempRoot);
    assert.strictEqual(noAccess.mode, 'manual_only', 'missing skill/credentials should force manual mode');
    assert.strictEqual(noAccess.cadence_hours, 24, 'manual mode should downshift to daily cadence');
  });

  withTempRoot((tempRoot) => {
    const collectorPath = path.join(
      tempRoot,
      'client',
      'cognition',
      'adaptive',
      'sensory',
      'eyes',
      'collectors',
      'moltbook_hot.ts'
    );
    const skillPath = path.join(tempRoot, 'client', 'cognition', 'skills', 'moltbook', 'moltbook_api.js');
    const credentialPath = path.join(tempRoot, 'client', 'runtime', 'config', 'moltbook', 'credentials.json');
    fs.mkdirSync(path.dirname(collectorPath), { recursive: true });
    fs.mkdirSync(path.dirname(skillPath), { recursive: true });
    fs.mkdirSync(path.dirname(credentialPath), { recursive: true });
    fs.writeFileSync(collectorPath, '// collector\n', 'utf8');
    fs.writeFileSync(skillPath, '// skill\n', 'utf8');
    fs.writeFileSync(credentialPath, '{"apiKey":"demo"}\n', 'utf8');

    process.env.PROTHEUS_REMINDER_ROOT = tempRoot;
    const ready = bridge.buildMoltcheckSnapshot(tempRoot);
    assert.strictEqual(ready.mode, 'automation_ready', 'collector+skill+credential should enable automation mode');
    assert.strictEqual(ready.cadence_hours, 4, 'automation mode should keep 4h cadence');
  });

  delete process.env.PROTHEUS_REMINDER_ROOT;
  console.log('reminder_data_bridge.test.js: OK');
} catch (err) {
  delete process.env.PROTHEUS_REMINDER_ROOT;
  console.error(`reminder_data_bridge.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
