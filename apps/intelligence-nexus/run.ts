#!/usr/bin/env node
'use strict';

const { runInfringOps } = require('../../client/runtime/systems/ops/run_infring_ops.ts');

const args = process.argv.slice(2);
const commandArgs = args.length === 0 ? ['open'] : args;
const exit = runInfringOps(['intelligence-nexus', ...commandArgs]);
process.exit(exit);
