#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const RUNNER_PATH = [
  path.join(ROOT, 'runtime', 'lib', 'ops_domain_conduit_runner.js'),
  path.join(ROOT, 'lib', 'ops_domain_conduit_runner.js')
].find((candidate) => fs.existsSync(candidate));

if (!RUNNER_PATH) {
  throw new Error('ops_domain_conduit_runner_missing');
}

const runner = require(RUNNER_PATH);

try {
  assert.ok(typeof runner.parseArgs === 'function', 'parseArgs export is required for regression tests');
  assert.ok(typeof runner.buildPassArgs === 'function', 'buildPassArgs export is required for regression tests');
  assert.ok(typeof runner.buildRunOptions === 'function', 'buildRunOptions export is required for regression tests');

  const viaFlag = runner.parseArgs(['--domain=spine', 'run', '--mode=daily', '--date=2026-03-09']);
  assert.deepStrictEqual(
    runner.buildPassArgs(viaFlag),
    ['run'],
    'domain supplied via --domain must preserve first command token'
  );

  const viaPositional = runner.parseArgs(['spine', 'run', '--mode=daily', '--date=2026-03-09']);
  assert.deepStrictEqual(
    runner.buildPassArgs(viaPositional),
    ['run'],
    'positional domain must be removed from payload args'
  );

  const defaults = runner.buildRunOptions(runner.parseArgs(['--domain=spine']));
  assert.strictEqual(defaults.skipRuntimeGate, true, 'runtime gate should default to skipped in compatibility lane');
  assert.strictEqual(defaults.stdioTimeoutMs, 120000, 'stdio timeout default should stay bounded');
  assert.strictEqual(defaults.timeoutMs, 125000, 'bridge timeout default should stay bounded and deterministic');

  const explicit = runner.buildRunOptions(
    runner.parseArgs(['--domain=spine', '--stdio-timeout-ms=50000', '--timeout-ms=61000', '--skip-runtime-gate=0'])
  );
  assert.strictEqual(explicit.skipRuntimeGate, false, 'explicit skip-runtime-gate=0 must be honored');
  assert.strictEqual(explicit.stdioTimeoutMs, 50000, 'explicit stdio timeout override must be honored');
  assert.strictEqual(explicit.timeoutMs, 61000, 'explicit bridge timeout override must be honored');

  console.log('ops_domain_conduit_runner.test.js: OK');
} catch (err) {
  console.error(`ops_domain_conduit_runner.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
