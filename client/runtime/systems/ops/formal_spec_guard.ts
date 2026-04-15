#!/usr/bin/env node

// TypeScript compatibility shim only.
// Layer ownership: core/layer1/policy + core/layer2/ops (authoritative)

'use strict';

const path = require('node:path');
const { invokeTsModuleSync } = require('../../lib/in_process_ts_delegate.ts');

const WORKSPACE_ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const GUARD_SCRIPT = path.resolve(WORKSPACE_ROOT, 'tests/tooling/scripts/ci/formal_spec_guard.ts');
const DEFAULT_COMMAND = 'run';

function normalizeArgs(argv = []) {
  return Array.isArray(argv) ? argv.map((v) => String(v || '')).filter(Boolean) : [];
}

function normalizeResult(res = {}) {
  const status = Number.isFinite(Number(res && res.status)) ? Number(res.status) : 1;
  const value = res && res.value && typeof res.value === 'object' ? res.value : null;
  const payload = value || {
    ok: status === 0,
    type: 'formal_spec_guard',
  };
  return {
    ...payload,
    status,
    delegated_to: 'tests/tooling/scripts/ci/formal_spec_guard.ts',
    stdout: String(res && res.stdout ? res.stdout : ''),
    stderr: String(res && res.stderr ? res.stderr : ''),
  };
}

function runDelegate(argv = [], tee = false) {
  const args = normalizeArgs(argv);
  const passArgs = args.length ? args : [DEFAULT_COMMAND];
  const res = invokeTsModuleSync(GUARD_SCRIPT, {
    argv: passArgs,
    cwd: WORKSPACE_ROOT,
    exportName: 'run',
    teeStdout: tee,
    teeStderr: tee,
  });
  return normalizeResult(res);
}

function run(argv = []) {
  return runDelegate(argv, false);
}

function runCli(argv = process.argv.slice(2)) {
  const result = runDelegate(argv, true);
  if (!result.stdout.trim()) {
    process.stdout.write(`${JSON.stringify(result)}\n`);
  }
  const status = Number(result.status);
  return Number.isFinite(status) ? status : (result.ok ? 0 : 1);
}

if (require.main === module) {
  process.exit(runCli(process.argv.slice(2)));
}

module.exports = {
  DEFAULT_COMMAND,
  normalizeArgs,
  run,
  runCli,
};
