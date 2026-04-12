#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ts_bootstrap.ts')).installTsRequireHook();

const mod = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ops_domain_conduit_runner.ts'));

function run() {
  const parsed = mod.parseArgs(['--domain', 'legacy-retired-lane', 'build', '--lane-id=FOO-1']);
  const args = mod.buildPassArgs(parsed);
  assert.deepStrictEqual(args, ['build', '--lane-id=FOO-1']);

  const parsedPositional = mod.parseArgs(['legacy-retired-lane', 'build', '--lane-id=FOO-2']);
  const positionalArgs = mod.buildPassArgs(parsedPositional);
  assert.deepStrictEqual(positionalArgs, ['build', '--lane-id=FOO-2']);

  const rawPassthrough = mod.buildPassArgs({ _: ['legacy-retired-lane', 'status'], verbose: true });
  assert.deepStrictEqual(rawPassthrough, ['status', '--verbose']);
}

run();
console.log(
  JSON.stringify({
    ok: true,
    type: 'ops_domain_conduit_runner_arg_passthrough_test'
  }),
);
