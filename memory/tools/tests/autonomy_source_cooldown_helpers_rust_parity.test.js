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

function run() {
  const ts = loadAutonomy(false);
  const rust = loadAutonomy(true);

  const capabilitySamples = ['eyes:collector', '  OPS-Lane ', '', null];
  for (const sample of capabilitySamples) {
    assert.strictEqual(
      rust.capabilityCooldownKey(sample),
      ts.capabilityCooldownKey(sample),
      `capabilityCooldownKey mismatch for ${String(sample)}`
    );
  }

  const readinessSamples = [
    ['strategy_a', 'execute'],
    ['strategy_b', 'canary_execute'],
    ['strategy_c', ''],
    ['', 'execute']
  ];
  for (const sample of readinessSamples) {
    assert.strictEqual(
      rust.readinessRetryCooldownKey(sample[0], sample[1]),
      ts.readinessRetryCooldownKey(sample[0], sample[1]),
      `readinessRetryCooldownKey mismatch for ${JSON.stringify(sample)}`
    );
  }

  const proposal = {
    meta: { source_eye: 'eye:market_watch' },
    evidence: [{ ref: 'eye:market_watch digest' }]
  };
  assert.strictEqual(
    rust.sourceEyeId(proposal),
    ts.sourceEyeId(proposal),
    'sourceEyeId mismatch'
  );

  const deprioritizedProposal = {
    meta: { source_eye: 'eye:docs_review' },
    evidence: [{ ref: 'eye:docs_review evidence' }]
  };
  assert.strictEqual(
    rust.isDeprioritizedSourceProposal(deprioritizedProposal),
    ts.isDeprioritizedSourceProposal(deprioritizedProposal),
    'isDeprioritizedSourceProposal mismatch'
  );

  const evidenceRefs = [
    'proof eye:collector_alpha result',
    'none',
    '',
    null
  ];
  for (const sample of evidenceRefs) {
    assert.strictEqual(
      rust.extractEyeFromEvidenceRef(sample),
      ts.extractEyeFromEvidenceRef(sample),
      `extractEyeFromEvidenceRef mismatch for ${String(sample)}`
    );
  }

  console.log('autonomy_source_cooldown_helpers_rust_parity.test.js: OK');
}

try {
  run();
} catch (err) {
  console.error(`autonomy_source_cooldown_helpers_rust_parity.test.js: FAIL: ${err.message}`);
  process.exit(1);
}

