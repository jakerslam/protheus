#!/usr/bin/env node
'use strict';

const { runInfringOps } = require('../../client/runtime/systems/ops/run_infring_ops.ts');

const args = process.argv.slice(2);
const commandArgs =
  args.length === 0
    ? ['status', '--app=code-engineer']
    : args;
const normalized =
  commandArgs.some((arg) => arg.startsWith('--app='))
    ? commandArgs
    : [commandArgs[0], '--app=code-engineer', ...commandArgs.slice(1)];

const exit = runInfringOps(['app-plane', ...normalized]);
process.exit(exit);
