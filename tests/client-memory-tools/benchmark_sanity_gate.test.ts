#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const SCRIPT = path.join(ROOT, 'tests', 'tooling', 'scripts', 'ci', 'benchmark_sanity_gate.ts');

function parseLastJson(stdout) {
  const whole = String(stdout || '').trim();
  if (whole) {
    try {
      return JSON.parse(whole);
    } catch {}
  }
  const lines = whole.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'benchmark-sanity-gate-'));
const policyPath = path.join(tempRoot, 'policy.json');
const statePath = path.join(tempRoot, 'state.json');

fs.writeFileSync(
  policyPath,
  `${JSON.stringify(
    {
      schema_version: '1.0',
      report_path: 'docs/client/reports/benchmark_matrix_stabilized_2026-03-18.json',
      runtime_source_report_path: 'docs/client/reports/benchmark_matrix_run_2026-03-06.json',
      state_path: statePath,
      required_projects: ['InfRing (rich)', 'InfRing (pure)', 'InfRing (tiny-max)'],
      required_metrics: ['cold_start_ms', 'idle_memory_mb', 'install_size_mb', 'tasks_per_sec'],
      bounds: {
        cold_start_ms: { min: 0.1, max: 5000 },
        idle_memory_mb: { min: 0.1, max: 500 },
        install_size_mb: { min: 0.1, max: 500 },
        tasks_per_sec: { min: 100, max: 1000000 },
      },
      max_step_multiplier: {
        cold_start_ms: 5.0,
        idle_memory_mb: 5.0,
        install_size_mb: 5.0,
        tasks_per_sec: 10.0,
      },
      step_change_exemptions: [],
      infring_required_runtime_source_keys: ['mode', 'tasks_source', 'tasks_sample_ms'],
    },
    null,
    2,
  )}\n`,
  'utf8',
);

const proc = spawnSync('node', [SCRIPT, `--policy=${policyPath}`, '--strict=1'], {
  cwd: ROOT,
  encoding: 'utf8',
});

assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout);
const payload = parseLastJson(proc.stdout);
assert(payload, 'expected JSON payload');
assert.strictEqual(payload.ok, true);
assert.strictEqual(payload.type, 'benchmark_sanity_gate');
assert.strictEqual(payload.summary.pass, true);
assert.strictEqual(payload.summary.required_rows, 12);
assert.strictEqual(payload.summary.measured_rows, 12);

const state = JSON.parse(fs.readFileSync(statePath, 'utf8'));
assert.strictEqual(
  state.source_report,
  'docs/client/reports/benchmark_matrix_stabilized_2026-03-18.json',
  'expected stabilized report as primary source',
);
assert.strictEqual(
  state.runtime_source_report,
  'docs/client/reports/benchmark_matrix_run_2026-03-06.json',
  'expected runtime source to remain the live report',
);
assert.strictEqual(state.report_type, 'competitive_benchmark_matrix_stabilized');
assert(state.projects['InfRing (rich)'], 'expected normalized rich project in state');
assert(state.projects['InfRing (tiny-max)'], 'expected normalized tiny-max project in state');

console.log('benchmark_sanity_gate.test.ts: OK');
