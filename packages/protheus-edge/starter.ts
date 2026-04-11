#!/usr/bin/env node
'use strict';

const edge = require('./index.ts');
const { parseArgs } = require('../../client/runtime/lib/queued_backlog_runtime');

const flags = parseArgs(process.argv.slice(2));
const mode = String(flags.mode || 'status').trim().toLowerCase();
const options = {
  owner: flags.owner,
  target: flags.target,
  edge: flags.edge,
  lifecycle: flags.lifecycle,
  cockpit: flags.cockpit,
  wrappers: flags.wrappers,
  benchmark: flags.benchmark,
  top: flags.top,
  max_mb: flags['max-mb'] || flags.max_mb,
  max_ms: flags['max-ms'] || flags.max_ms,
  policy: flags.policy
};

let out;
if (mode === 'contract') out = edge.edgeContract(options);
else if (mode === 'edge') out = edge.edgeRuntime('status', options);
else out = edge.edgeStatusBundle(options);

process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
process.exit(out && out.ok === false ? 1 : 0);
