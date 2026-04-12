#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
require(path.join(ROOT, 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const mod = require(path.join(ROOT, 'client', 'runtime', 'lib', 'duality_seed.ts'));
const TS_ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');
const DUALITY_TS = path.join(ROOT, 'client', 'runtime', 'lib', 'duality_seed.ts');

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  for (const line of trimmed.split('\n').reverse()) {
    const candidate = line.trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function main() {
  const cli = spawnSync('node', [TS_ENTRYPOINT, DUALITY_TS, 'status'], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  assert.equal(cli.status, 0, cli.stderr || cli.stdout || 'duality_seed status failed');
  const wrapperStatus = parseJsonOutput(cli.stdout) || parseJsonOutput(cli.stderr);
  const status = wrapperStatus && wrapperStatus.payload && wrapperStatus.payload.payload
    ? wrapperStatus.payload.payload
    : null;
  assert(wrapperStatus && wrapperStatus.ok === true, 'expected duality seed wrapper receipt');
  assert(status && status.ok === true, 'expected duality seed status receipt');
  assert.equal(wrapperStatus.type, 'ops_domain_conduit_runner_kernel');
  assert.equal(status.type, 'duality_seed_status');
  assert.deepStrictEqual(status.commands, ['status', 'invoke']);

  const codex = mod.parseDualityCodexText(`
Flux Pairs: Yin/Yang
Flow Values: observe/adapt, observe/adapt; order/chaos
Warnings: gentle caution
`);
  assert.deepStrictEqual(codex.flow_values, [
    'life/death',
    'progression/degression',
    'creation/decay',
    'integration/fragmentation',
  ]);
  assert.equal(Array.isArray(codex.flux_pairs), true);
  assert.equal(codex.version, '1.0');

  const evaluation = mod.evaluateDualitySignal({
    lane: 'web/collector',
    objective: 'keep order and exploration in harmony with safety and creativity',
  });
  assert.equal(evaluation.enabled, true);
  assert.equal(evaluation.lane, 'web/collector');
  assert.equal(evaluation.diagnostics.source, 'runtime');
  assert.equal(typeof evaluation.zero_point_harmony_potential, 'number');
  assert.ok(['ok', 'pain', 'unknown'].includes(String(evaluation.score_label || '')));

  console.log(JSON.stringify({ ok: true, type: 'duality_seed_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
