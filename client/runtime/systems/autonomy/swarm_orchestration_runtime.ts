#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (swarm orchestration coordination); this file is a thin CLI bridge.
// SRS contract evidence anchor: V6-SWARM-032 (client-settable token budget bridge).

const impl = require('../../../../surface/orchestration/scripts/swarm_orchestration_runtime.ts');

function run(argv = process.argv.slice(2)) {
  return impl.run(argv);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  ...impl,
  run,
};
