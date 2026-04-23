#!/usr/bin/env node
'use strict';
export {};

// TypeScript compatibility shim only.
// Layer ownership: core/layer0/ops (authoritative); this file is wrapper-only.

// Thin systems entrypoint for One Knowledge bridge.

const { runCli } = require('../../lib/one_knowledge.ts');

process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK = process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK || '0';
process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK =
  process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK || '0';

function normalizeArgs(argv) {
  if (!Array.isArray(argv)) return [];
  return argv.map((token) => String(token || '').trim()).filter(Boolean);
}

function mapCommandAliases(args) {
  if (!Array.isArray(args) || args.length === 0) return [];
  const first = String(args[0] || '').trim().toLowerCase();
  const map = {
    'sync-catalog': 'sync',
    import: 'import-flow',
    execute: 'run-flow',
    run: 'run-flow',
    'connect-oauth': 'connect'
  };
  const mapped = map[first] || first;
  return [mapped].concat(args.slice(1));
}

function run(argv = process.argv.slice(2)) {
  const args = mapCommandAliases(normalizeArgs(argv));
  return runCli(args);
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { normalizeArgs, mapCommandAliases, run };
