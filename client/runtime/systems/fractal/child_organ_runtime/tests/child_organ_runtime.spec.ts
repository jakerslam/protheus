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

const mod = require('../../child_organ_runtime.ts');

assert.deepStrictEqual(
  mod.mapArgs(['child_organ_runtime', 'run', '--organ-id=a', '--command=true']),
  ['spawn', '--organ-id=a', '--command=true']
);
assert.deepStrictEqual(
  mod.mapArgs(['prepare', '--organ-id=a']),
  ['plan', '--organ-id=a']
);
assert.deepStrictEqual(
  mod.mapArgs(['\u200Bunknown', '--organ-id=a']),
  ['status', '--organ-id=a']
);
assert.deepStrictEqual(mod.mapArgs([]), ['status']);

const out = mod.ensureMutationReceipt({ payload: { ok: true, type: 'child_organ_runtime_plan' } }, 'plan');
assert.ok(typeof out.payload.receipt_hash === 'string');
assert.equal(out.payload.receipt_hash.length >= 64, true);

const nonMut = mod.ensureMutationReceipt({ payload: { ok: true, type: 'child_organ_runtime_status' } }, 'status');
assert.equal(nonMut.payload.receipt_hash, undefined);

console.log('child_organ_runtime wrapper checks passed');
