#!/usr/bin/env node
'use strict';

const path = require('path');
const assert = require('assert');

const REPO_ROOT = path.resolve(__dirname, '..', '..', '..');
const autonomyPath = path.join(REPO_ROOT, 'systems', 'autonomy', 'autonomy_controller.js');
const bridgePath = path.join(REPO_ROOT, 'systems', 'autonomy', 'backlog_autoscale_rust_bridge.js');

function loadAutonomy(rustEnabled) {
  process.env.AUTONOMY_BACKLOG_AUTOSCALE_RUST_ENABLED = rustEnabled ? '1' : '0';
  delete require.cache[autonomyPath];
  delete require.cache[bridgePath];
  return require(autonomyPath);
}

function approxEqual(a, b, epsilon = 1e-3) {
  return Math.abs(Number(a) - Number(b)) <= epsilon;
}

function run() {
  const ts = loadAutonomy(false);
  const rust = loadAutonomy(true);

  const tokenSamples = ['revenue', ' delivery ', 'unknown', '', null];
  for (const sample of tokenSamples) {
    assert.strictEqual(
      rust.normalizeValueCurrencyToken(sample),
      ts.normalizeValueCurrencyToken(sample),
      `normalizeValueCurrencyToken mismatch for ${String(sample)}`
    );
  }

  const listSamples = [
    ['revenue', 'delivery', 'revenue'],
    'user_value, quality, invalid, time_savings',
    [],
    ''
  ];
  for (const sample of listSamples) {
    assert.deepStrictEqual(
      rust.listValueCurrencies(sample),
      ts.listValueCurrencies(sample),
      `listValueCurrencies mismatch for ${JSON.stringify(sample)}`
    );
  }

  const inferBits = [
    'Ship backlog faster with better delivery throughput',
    'Improve retention and onboarding quality',
    'Research hypothesis to learn new insights'
  ];
  assert.deepStrictEqual(
    rust.inferValueCurrenciesFromDirectiveBits(inferBits),
    ts.inferValueCurrenciesFromDirectiveBits(inferBits),
    'inferValueCurrenciesFromDirectiveBits mismatch'
  );

  const linkedEntry = {
    objective_id: 'T1_build_runtime',
    directive_objective_id: '',
    directive: 'N/A'
  };
  assert.strictEqual(
    rust.hasLinkedObjectiveEntry(linkedEntry),
    ts.hasLinkedObjectiveEntry(linkedEntry),
    'hasLinkedObjectiveEntry mismatch'
  );

  const outcomeEntry = { outcome_verified: false, outcome: 'verified_success' };
  assert.strictEqual(
    rust.isVerifiedEntryOutcome(outcomeEntry),
    ts.isVerifiedEntryOutcome(outcomeEntry),
    'isVerifiedEntryOutcome mismatch'
  );

  const revenueAction = { verified: false, outcome_verified: false, status: 'received' };
  assert.strictEqual(
    rust.isVerifiedRevenueAction(revenueAction),
    ts.isVerifiedRevenueAction(revenueAction),
    'isVerifiedRevenueAction mismatch'
  );

  const nowMs = Date.UTC(2026, 2, 4, 12, 15, 0, 0);
  assert.strictEqual(
    rust.minutesUntilNextUtcDay(nowMs),
    ts.minutesUntilNextUtcDay(nowMs),
    'minutesUntilNextUtcDay mismatch'
  );

  const tsAge = ts.ageHours('2026-03-03');
  const rustAge = rust.ageHours('2026-03-03');
  assert(
    approxEqual(tsAge, rustAge, 0.01),
    `ageHours mismatch: ts=${tsAge} rust=${rustAge}`
  );

  const url = 'https://example.com/path?q=1';
  assert.strictEqual(rust.urlDomain(url), ts.urlDomain(url), 'urlDomain mismatch');

  const allowlist = ['example.com', 'acme.dev'];
  assert.strictEqual(
    rust.domainAllowed('sub.example.com', allowlist),
    ts.domainAllowed('sub.example.com', allowlist),
    'domainAllowed mismatch'
  );

  console.log('autonomy_support_primitives_rust_parity.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`autonomy_support_primitives_rust_parity.test.js: FAIL: ${err.message}`);
  process.exit(1);
}
