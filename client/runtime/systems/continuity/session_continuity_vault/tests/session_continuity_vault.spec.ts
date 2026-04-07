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

const mod = require('../../session_continuity_vault.ts');

const withAliases = mod.normalizeArgs([
  'session_continuity_vault',
  'restore',
  '--session-id=alpha'
]);
assert.deepStrictEqual(withAliases, ['get', '--session-id=alpha']);

const withStatus = mod.normalizeArgs([]);
assert.deepStrictEqual(withStatus, ['status']);

const wrapped = mod.ensureMutationReceipt({ payload: { ok: true, type: 'session_continuity_vault_put' } }, 'put');
assert.ok(typeof wrapped.payload.receipt_hash === 'string');
assert.ok(wrapped.payload.receipt_hash.length >= 64);

console.log('session_continuity_vault wrapper checks passed');
