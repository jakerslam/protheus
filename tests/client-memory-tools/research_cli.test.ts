#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const CLIENT_WRAPPER = path.join(ROOT, 'client/runtime/systems/research/research_organ.ts');
const SURFACE_WRAPPER = path.join(ROOT, 'surface/orchestration/scripts/research_organ.ts');

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {}
  const lines = trimmed.split('\n');
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    const candidate = lines[index].trim();
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
  assert.equal(proc.status, 0, proc.stderr || proc.stdout || `failed:${scriptPath}`);
  const payload = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
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

  assert.equal(clientStatus.system_id, 'SYSTEMS-RESEARCH-RESEARCH_ORGAN');
  assert.equal(clientStatus.type, 'runtime_systems_status');
  assert.equal(clientStatus.command, 'status');
  assert.equal(clientStatus.lane, 'runtime_systems');
  assert.equal(surfaceStatus.system_id, 'SYSTEMS-RESEARCH-RESEARCH_ORGAN');
  assert.equal(surfaceStatus.type, 'runtime_systems_status');
  assert.equal(surfaceStatus.command, 'status');
  assert.equal(surfaceStatus.lane, 'runtime_systems');
  assert.equal(clientStatus.latest_path, surfaceStatus.latest_path);
  assert.equal(clientStatus.history_path, surfaceStatus.history_path);
  assert.equal(typeof clientStatus.receipt_hash, 'string');
  assert.equal(typeof surfaceStatus.receipt_hash, 'string');

  console.log(JSON.stringify({ ok: true, type: 'research_cli_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
