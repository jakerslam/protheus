#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (routing coordination); this file is a thin CLI bridge.

const impl = require('../../../../surface/orchestration/scripts/llm_gateway_failure_classifier.ts');
const BRIDGE_PATH = 'client/runtime/systems/routing/llm_gateway_failure_classifier.ts';
const ORCHESTRATION_SCRIPT = 'surface/orchestration/scripts/llm_gateway_failure_classifier.ts';
const MODULE_KEY = 'llm_gateway_failure_classifier';

function normalizeArgs(args = []) {
  return Array.isArray(args) ? args.map((value) => String(value)) : [];
}

function run(args = process.argv.slice(2)) {
  return impl.run(normalizeArgs(args));
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  BRIDGE_PATH,
  ORCHESTRATION_SCRIPT,
  MODULE_KEY,
  normalizeArgs,
  ...impl,
  run
};
