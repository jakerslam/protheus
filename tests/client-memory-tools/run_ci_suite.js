#!/usr/bin/env node
'use strict';

const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../..');
const SUITE = [
  'tests/client-memory-tools/memory_security_gate_integration.test.js',
  'tests/client-memory-tools/memory_uid_enforcement.test.js',
  'tests/client-memory-tools/memory_recall_context_budget.test.js',
  'tests/client-memory-tools/memory_index_freshness_gate.test.js',
  'tests/client-memory-tools/v6_memory_013_019_client_regression.test.js'
];

function runOne(testPath) {
  const absolutePath = path.resolve(ROOT, testPath);
  const proc = spawnSync(process.execPath, [absolutePath], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  return {
    test: testPath,
    status: Number.isFinite(Number(proc.status)) ? Number(proc.status) : 1,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || '')
  };
}

function main() {
  const results = [];
  for (const testPath of SUITE) {
    const result = runOne(testPath);
    results.push(result);
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
  }

  const failed = results.filter((row) => row.status !== 0);
  const payload = {
    ok: failed.length === 0,
    type: 'client_memory_ci_suite',
    total: results.length,
    failed: failed.length,
    failures: failed.map((row) => ({ test: row.test, status: row.status }))
  };
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  process.exit(payload.ok ? 0 : 1);
}

if (require.main === module) {
  main();
}

module.exports = {
  SUITE,
  runOne
};
