#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../../..');
const OUT_PATH = 'core/local/artifacts/arch_boundary_conformance_test_current.json';

function parseJsonPayload(raw) {
  const text = String(raw || '').trim();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {}
  const lines = text.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const candidate = lines.slice(index).join('\n').trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function main() {
  const proc = spawnSync(
    'node',
    [
      'client/runtime/lib/ts_entrypoint.ts',
      'tests/tooling/scripts/ci/arch_boundary_conformance.ts',
      '--strict=1',
      `--out=${OUT_PATH}`,
    ],
    {
      cwd: ROOT,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );

  const payload = parseJsonPayload(proc.stdout) || parseJsonPayload(proc.stderr);
  if (proc.status !== 0) {
    throw new Error(
      `arch_boundary_conformance_failed:status=${proc.status}:stderr=${String(proc.stderr || '').trim()}`,
    );
  }
  assert(payload && typeof payload === 'object', 'missing_arch_boundary_payload');
  assert.strictEqual(payload.type, 'arch_boundary_conformance');
  assert(payload.summary && typeof payload.summary === 'object', 'missing_arch_boundary_summary');
  assert.strictEqual(payload.summary.hard_violation_count, 0);
  assert.strictEqual(payload.summary.allowed_violation_count, 0);
  assert.strictEqual(payload.summary.pass, true);

  console.log(JSON.stringify({ ok: true, type: 'orchestration_boundary_conformance_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
