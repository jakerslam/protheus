#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');

const root = path.resolve(__dirname, '..', '..');
const script = path.join(root, 'client', 'runtime', 'systems', 'tools', 'assimilate.js');
const runOpsScript = path.join(root, 'client', 'runtime', 'systems', 'ops', 'run_protheus_ops.js');

function run(args) {
  const out = spawnSync(process.execPath, [script, ...args], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 1024 * 1024 * 8,
  });
  return out;
}

function parseJson(stdout) {
  return JSON.parse(String(stdout || '').trim());
}

function runProtheusAssimilate(args) {
  return spawnSync(process.execPath, [runOpsScript, 'protheusctl', 'assimilate', ...args], {
    cwd: root,
    encoding: 'utf8',
    maxBuffer: 1024 * 1024 * 8,
  });
}

// 1) scaffold contract for known target
const scaffold = run(['haystack', '--scaffold-payload=1']);
assert.strictEqual(scaffold.status, 0, scaffold.stderr || 'scaffold failed');
const scaffoldJson = parseJson(scaffold.stdout);
assert.strictEqual(scaffoldJson.ok, true);
assert.strictEqual(scaffoldJson.type, 'assimilate_payload_scaffold');
assert.strictEqual(scaffoldJson.target, 'haystack');
assert.ok(Array.isArray(scaffoldJson.payload.components));
assert.ok(scaffoldJson.payload_base64 && typeof scaffoldJson.payload_base64 === 'string');

// 2) simulation path emits runtime metrics in JSON
const simulation = run(['codex', '--duration-ms=0', '--json=1']);
assert.strictEqual(simulation.status, 0, simulation.stderr || 'simulation failed');
const simulationJson = parseJson(simulation.stdout);
assert.strictEqual(simulationJson.ok, true);
assert.strictEqual(simulationJson.mode, 'simulation');
assert.ok(simulationJson.metrics);
assert.ok(typeof simulationJson.metrics.p50_ms === 'number');
assert.ok(typeof simulationJson.metrics.p95_ms === 'number');

// 3) runtime bridge path emits route + receipt + metrics
const runtime = runProtheusAssimilate([
  'dspy',
  '--payload-base64=e30=',
  '--strict=1',
  '--duration-ms=0',
  '--json=1',
]);
assert.strictEqual(runtime.status, 0, runtime.stderr || 'runtime assimilation failed');
const runtimeJson = parseJson(runtime.stdout);
assert.strictEqual(runtimeJson.ok, true);
assert.strictEqual(runtimeJson.mode, 'runtime');
assert.ok(runtimeJson.route);
assert.strictEqual(runtimeJson.route.domain, 'dspy-bridge');
assert.ok(
  typeof runtimeJson.receipt === 'string' &&
    (runtimeJson.receipt.startsWith('sha256:') || /^[a-f0-9]{64}$/i.test(runtimeJson.receipt)),
);
assert.ok(runtimeJson.metrics);
assert.ok(typeof runtimeJson.metrics.p50_ms === 'number');
assert.ok(typeof runtimeJson.metrics.p95_ms === 'number');

process.stdout.write('ok\n');
