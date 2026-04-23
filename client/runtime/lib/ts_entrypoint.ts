#!/usr/bin/env node
'use strict';

const path = require('path');
const Module = require('module');

const { bootstrap } = require('./ts_bootstrap.ts');

function usage() {
  process.stderr.write('Usage: node client/runtime/lib/ts_entrypoint.ts <target.ts> [args...]\n');
  process.stderr.write('   or set INFRING_TS_ENTRY_TARGET / INFRING_TS_ENTRY_TARGET\n');
}

function resolveEntrypointTarget(rawTarget) {
  const cliTarget = String(rawTarget || '').trim();
  if (cliTarget) return cliTarget;
  const preferred = String(process.env.INFRING_TS_ENTRY_TARGET || '').trim();
  const legacy = String(process.env.INFRING_TS_ENTRY_TARGET || '').trim();
  if (!preferred && legacy) {
    process.env.INFRING_TS_ENTRY_TARGET = legacy;
    return legacy;
  }
  if (preferred && !legacy) {
    process.env.INFRING_TS_ENTRY_TARGET = preferred;
  }
  return preferred;
}

function main() {
  const target = resolveEntrypointTarget(process.argv[2]);
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
