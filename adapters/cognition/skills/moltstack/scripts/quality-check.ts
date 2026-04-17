#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer3/cognition + core/layer0/ops::legacy-retired-lane (authoritative)
// Thin compatibility wrapper only.
const BRIDGE = '../../../lib/legacy_retired_wrapper.ts';

function loadBridge() {
  try {
    return require(BRIDGE);
  } catch (error) {
    return {
      ok: false,
      error: 'quality-check_bridge_load_failed',
      detail: String(error && error.message ? error.message : error || 'unknown_error'),
    };
  }
}

const runtime = loadBridge();
const mod =
  runtime && typeof runtime.createCognitionModule === 'function'
    ? runtime.createCognitionModule(__dirname, 'quality-check', 'COGNITION-SKILLS-MOLTSTACK-SCRIPTS-QUALITY-CHECK')
    : { ok: false, error: 'quality-check_bridge_missing_createCognitionModule' };

if (require.main === module) {
  if (mod && mod.ok === false) {
    process.stderr.write(JSON.stringify(mod) + '\n');
    process.exit(1);
  }
  if (!runtime || typeof runtime.runAsMain !== 'function') {
    process.stderr.write(JSON.stringify({ ok: false, error: 'quality-check_bridge_missing_runAsMain' }) + '\n');
    process.exit(1);
  }
  runtime.runAsMain(mod, process.argv.slice(2));
}

module.exports = mod;
