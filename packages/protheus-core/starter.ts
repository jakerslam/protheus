#!/usr/bin/env node
'use strict';

const core = require('./index.ts');
const { parseArgs } = require('../../client/runtime/lib/queued_backlog_runtime');

const flags = parseArgs(process.argv.slice(2));
const mode = String(flags.mode || '').trim().toLowerCase();
const options = {
  spine: flags.spine,
  reflex: flags.reflex,
  gates: flags.gates,
  timeout_ms: flags['timeout-ms'] || flags.timeout_ms,
  max_mb: flags['max-mb'] || flags.max_mb,
  max_ms: flags['max-ms'] || flags.max_ms,
};

const out = mode === 'contract'
  ? core.coldStartContract(options)
  : core.coreStatus(options);

process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
process.exit(out.ok ? 0 : 1);
