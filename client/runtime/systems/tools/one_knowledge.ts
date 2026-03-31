#!/usr/bin/env node
'use strict';
export {};

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative); this file is wrapper-only.

// Thin systems entrypoint for One Knowledge bridge.

const { runCli } = require('../../lib/one_knowledge.ts');

function run(argv = process.argv.slice(2)) {
  return runCli(Array.isArray(argv) ? argv : []);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
