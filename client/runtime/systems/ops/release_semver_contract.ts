#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops::release-semver-contract (authoritative)
// Thin TypeScript launcher wrapper only.

const { runProtheusOps } = require('./run_protheus_ops.ts');
const DEFAULT_COMMAND = 'status';
const ALLOWED_COMMANDS = new Set(['status', 'run']);

function normalizeArgs(argv = process.argv.slice(2)) {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function normalizeCommand(raw) {
  const token = String(raw || '').trim().toLowerCase();
  if (!token || token.startsWith('--')) return DEFAULT_COMMAND;
  return ALLOWED_COMMANDS.has(token) ? token : DEFAULT_COMMAND;
}

function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const command = normalizeCommand(args[0]);
  const rest =
    command === DEFAULT_COMMAND && (args[0] || '').startsWith('--') ? args : args.slice(1);
  const passArgs =
    command === DEFAULT_COMMAND && !ALLOWED_COMMANDS.has(String(args[0] || '').toLowerCase())
      ? [command, ...args]
      : [command, ...rest];
  return runProtheusOps(
    ['release-semver-contract', ...passArgs],
    {
      // Route through conduit kernel; direct local-core dispatch omits this domain key.
      preferLocalCore: false,
      env: {
        // Local core is authoritative for newest domains; prebuilt may lag.
        PROTHEUS_OPS_USE_PREBUILT: process.env.PROTHEUS_OPS_USE_PREBUILT || '0',
        PROTHEUS_OPS_LOCAL_TIMEOUT_MS: process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '600000',
      },
    }
  );
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  normalizeArgs,
  normalizeCommand,
  run,
};
