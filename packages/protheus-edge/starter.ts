#!/usr/bin/env node
'use strict';

const edge = require('./index.js');
const { parseArgs } = require('../../client/runtime/lib/queued_backlog_runtime');

const flags = parseArgs(process.argv.slice(2));
const mode = String(flags.mode || 'status').trim().toLowerCase();
const options = {
  owner: flags.owner,
  max_mb: flags['max-mb'] || flags.max_mb,
  max_ms: flags['max-ms'] || flags.max_ms,
  policy: flags.policy
};

let out;
if (mode === 'edge') out = edge.edgeRuntime('start', { ...options, apply: 0 });
else if (mode === 'contract') out = edge.edgeContract(options);
else out = edge.edgeStatusBundle(options);

process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
process.exit(out && out.ok === false ? 1 : 0);
