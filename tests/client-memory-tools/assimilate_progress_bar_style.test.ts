#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawnSync } = require('child_process');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const root = path.resolve(__dirname, '..', '..');
const script = path.join(root, 'client', 'runtime', 'systems', 'tools', 'assimilation_cli_bridge.ts');

const run = spawnSync(process.execPath, [script, 'codex', '--duration-ms=0'], {
  cwd: root,
  encoding: 'utf8',
});

assert.strictEqual(run.status, 0, run.stderr || 'assimilate command failed');
const output = run.stdout;

assert.ok(output.includes('Spinning up swarm (5,000 agents)'));
assert.ok(output.includes('Parallel analysis (manifest + docs)'));
assert.ok(output.includes('Building bridges & adapters'));
assert.ok(output.includes('Validating + signing receipts'));
assert.ok(output.includes('Assimilation complete. Ready to use.'));

// README-style benchmark bars use filled/empty block glyphs.
assert.ok(output.includes('█'));
assert.ok(output.includes('░'));

assert.ok(output.includes('Receipt: sha256:'));
assert.ok(output.includes('Target: codex fully assimilated. Agents online.'));
assert.ok(output.includes('Power to The Users.'));
assertNoPlaceholderOrPromptLeak({ output }, 'assimilate_progress_bar_style_test');
assertStableToolingEnvelope({
  ok: run.status === 0,
  status: run.status === 0 ? 'ok' : 'error',
  reason: run.stderr || run.stdout || 'assimilation_output_missing',
}, 'assimilate_progress_bar_style_test');

process.stdout.write('ok\n');
