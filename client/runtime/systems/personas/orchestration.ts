#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (persona orchestration coordination); this file is a thin CLI bridge.

const impl = require('../../../../surface/orchestration/scripts/personas_orchestration.ts');

function run(args = process.argv.slice(2)) {
  return impl.run(args);
}

if (require.main === module) {
  const result = run(process.argv.slice(2));
  if (typeof result === 'number' && Number.isFinite(result)) {
    process.exit(result);
  }
  if (result && typeof result === 'object' && typeof result.ok === 'boolean') {
    process.exit(result.ok ? 0 : 1);
  }
  process.exit(0);
}

module.exports = {
  ...impl,
  run
};
