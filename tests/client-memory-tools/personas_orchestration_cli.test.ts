#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const CLIENT_WRAPPER = path.join(ROOT, 'client/runtime/systems/personas/orchestration.ts');
const SURFACE_WRAPPER = path.join(ROOT, 'surface/orchestration/scripts/personas_orchestration.ts');

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {}
  const lines = trimmed.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const candidate = lines.slice(index).join('\n').trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function runScript(scriptPath, args = []) {
  const proc = spawnSync(process.execPath, [ENTRYPOINT, scriptPath, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  const payload = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout || `failed:${scriptPath}`);
  assert(payload && payload.ok === true, `expected ok payload from ${path.basename(scriptPath)}`);
  return payload;
}

function runtimePayload(out) {
  assert(out && out.payload && out.payload.ok === true, 'expected conduit payload envelope');
  const payload = out.payload.payload;
  assert(payload && payload.ok === true, 'expected runtime systems payload');
  return payload;
}

function main() {
  const clientStatus = runtimePayload(runScript(CLIENT_WRAPPER, ['status']));
  const surfaceStatus = runtimePayload(runScript(SURFACE_WRAPPER, ['status']));

  assert.strictEqual(clientStatus.system_id, 'SYSTEMS-PERSONAS-ORCHESTRATION');
  assert.strictEqual(clientStatus.type, 'runtime_systems_status');
  assert.strictEqual(clientStatus.command, 'status');
  assert.strictEqual(clientStatus.lane, 'runtime_systems');
  assert.strictEqual(clientStatus.latest_path, surfaceStatus.latest_path);
  assert.strictEqual(clientStatus.history_path, surfaceStatus.history_path);
  assert.strictEqual(typeof clientStatus.receipt_hash, 'string');
  assert.strictEqual(typeof surfaceStatus.receipt_hash, 'string');

  const meeting = runtimePayload(
    runScript(CLIENT_WRAPPER, ['meeting', 'Plan next swarm audit', '--dry-run=1', '--strict=1']),
  );
  const project = runtimePayload(
    runScript(SURFACE_WRAPPER, ['project', 'Harden lineage receipts', '--dry-run=1']),
  );

  assert.strictEqual(meeting.system_id, 'SYSTEMS-PERSONAS-ORCHESTRATION');
  assert.strictEqual(meeting.command, 'meeting');
  assert.strictEqual(meeting.strict, true);
  assert.strictEqual(project.command, 'project');
  assert.strictEqual(project.lane, 'runtime_systems');
  assert(Array.isArray(meeting.claim_evidence) && meeting.claim_evidence.length >= 1);
  assert(
    meeting.claim_evidence.some((row) => row && row.id === 'runtime_system_mutation_receipted'),
    'expected deterministic runtime mutation receipt claim evidence',
  );
  assert.strictEqual(fs.existsSync(path.join(ROOT, meeting.latest_path)), true);
  assert.strictEqual(fs.existsSync(path.join(ROOT, meeting.history_path)), true);

  console.log(JSON.stringify({ ok: true, type: 'personas_orchestration_cli_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
