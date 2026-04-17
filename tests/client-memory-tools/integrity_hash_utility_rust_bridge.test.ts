#!/usr/bin/env node
'use strict';

const assert = require('assert');
const crypto = require('crypto');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const mod = require('../../client/runtime/lib/integrity_hash_utility.ts');

const serialized = mod.stableStringify({ b: 2, a: 1, nested: { z: 2, y: 1 } });
assert.strictEqual(serialized, '{"a":1,"b":2,"nested":{"y":1,"z":2}}');

const digest = mod.sha256Hex({ b: 2, a: 1, nested: { z: 2, y: 1 } });
const expected = crypto.createHash('sha256').update(serialized, 'utf8').digest('hex');
assert.strictEqual(digest, expected);

const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'integrity-hash-utility-'));
const filePath = path.join(tempDir, 'sample.txt');
fs.writeFileSync(filePath, 'hello world\n', 'utf8');
assert.strictEqual(
  mod.hashFileSha256(filePath),
  crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex')
);
fs.rmSync(tempDir, { recursive: true, force: true });
assertNoPlaceholderOrPromptLeak({ serialized, digest }, 'integrity_hash_utility_rust_bridge_test');
assertStableToolingEnvelope({ status: 'ok', digest }, 'integrity_hash_utility_rust_bridge_test');

console.log(JSON.stringify({ ok: true, type: 'integrity_hash_utility_rust_bridge_test' }));
