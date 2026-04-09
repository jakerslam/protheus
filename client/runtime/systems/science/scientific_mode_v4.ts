#!/usr/bin/env node
'use strict';
// TypeScript compatibility shim only.
// Layer ownership: surface/orchestration (science coordination); this file is a thin CLI bridge.

const impl = require('../../../../surface/orchestration/scripts/scientific_mode_v4.ts');

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
