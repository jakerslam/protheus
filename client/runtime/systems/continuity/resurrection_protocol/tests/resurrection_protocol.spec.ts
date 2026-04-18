#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');

if (!require.extensions['.ts']) {
  const ts = require('typescript');
  require.extensions['.ts'] = function compile(module, filename) {
    const source = require('node:fs').readFileSync(filename, 'utf8');
    const output = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        esModuleInterop: true,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        skipLibCheck: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(output, filename);
  };
}

const mod = require('../../resurrection_protocol.ts');

assert.deepStrictEqual(
  mod.normalizeArgs(['resurrection-protocol', 'run', '--session-id=a']),
  ['checkpoint', '--session-id=a']
);
assert.deepStrictEqual(
  mod.normalizeArgs(['resurrection_protocol', 'resurrection-protocol', 'build', '--session-id=a']),
  ['checkpoint', '--session-id=a']
);
assert.deepStrictEqual(
  mod.normalizeArgs(['build', '--session-id=a']),
  ['checkpoint', '--session-id=a']
);
assert.deepStrictEqual(
  mod.normalizeArgs(['\u200Bunknown', '--session-id=a']),
  ['status']
);
assert.deepStrictEqual(mod.normalizeArgs([]), ['status']);

const receipt = mod.ensureMutationReceipt({ payload: { ok: true, type: 'resurrection_protocol_checkpoint' } }, 'checkpoint');
assert.ok(typeof receipt.payload.receipt_hash === 'string');
assert.equal(receipt.payload.receipt_hash.length >= 64, true);

const applyStatus = mod.ensureMutationReceipt(
  { payload: { ok: true, type: 'resurrection_protocol_status', apply: true } },
  'status'
);
assert.ok(typeof applyStatus.payload.receipt_hash === 'string');

const noMut = mod.ensureMutationReceipt({ payload: { ok: true, type: 'resurrection_protocol_status' } }, 'status');
assert.equal(noMut.payload.receipt_hash, undefined);

console.log('resurrection_protocol wrapper checks passed');
