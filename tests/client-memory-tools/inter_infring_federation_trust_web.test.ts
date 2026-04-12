#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const TARGET = path.join(ROOT, 'tests', 'client-memory-tools', 'inter_protheus_federation_trust_web.test.ts');

const proc = spawnSync(process.execPath, [TARGET], {
  cwd: ROOT,
  encoding: 'utf8'
});
assert.equal(proc.status, 0, proc.stderr || proc.stdout);
assert.match(proc.stdout, /inter_protheus_federation_trust_web_test/);

console.log(JSON.stringify({ ok: true, type: 'inter_infring_federation_trust_web_test' }));
