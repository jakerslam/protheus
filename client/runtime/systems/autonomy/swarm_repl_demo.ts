#!/usr/bin/env node
'use strict';

// TypeScript compatibility shim only.
// Layer ownership: adapters/runtime::swarm-repl-demo (authoritative bridge-driven demo shell).

const mod = require('../../../../adapters/runtime/swarm_repl_demo.ts');

if (require.main === module) {
  try {
    process.exit(mod.run(process.argv.slice(2)));
  } catch (error) {
    process.stderr.write(`${String(error && error.message ? error.message : error)}\n`);
    process.exit(1);
  }
}

module.exports = mod;
