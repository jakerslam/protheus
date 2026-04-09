#!/usr/bin/env node
'use strict';

const path = require('path');
const Module = require('module');

const { bootstrap } = require('./ts_bootstrap.ts');

function usage() {
  process.stderr.write('Usage: node client/runtime/lib/ts_entrypoint.ts <target.ts> [args...]\n');
}

function main() {
  const target = String(process.argv[2] || '').trim();
  if (!target) {
    usage();
    process.exit(2);
  }
  const targetTs = path.resolve(target);
  if (!targetTs.endsWith('.ts')) {
    process.stderr.write(`ts_entrypoint: target must be .ts: ${targetTs}\n`);
    process.exit(2);
  }
  const forwardedArgs = process.argv.slice(3);
  process.argv = [process.argv[0], targetTs, ...forwardedArgs];

  const entry = new Module(targetTs, module.parent || module);
  entry.id = '.';
  entry.filename = targetTs;
  entry.paths = Module._nodeModulePaths(path.dirname(targetTs));
  require.main = entry;
  process.mainModule = entry;
  bootstrap(targetTs, entry);
}

main();
