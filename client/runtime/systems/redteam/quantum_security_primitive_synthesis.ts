#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: orchestration (redteam coordination); this file is a thin CLI bridge.

const impl = require('../../../../orchestration/scripts/quantum_security_primitive_synthesis.ts');

function run(args = process.argv.slice(2)) {
  return impl.run(args);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  run
};
