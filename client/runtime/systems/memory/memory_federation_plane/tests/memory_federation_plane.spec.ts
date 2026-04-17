#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const FORBIDDEN_RUNTIME_CONTEXT_MARKERS = [
  'You are an expert Python programmer.',
  '[PATCH v2',
  'List Leaves (25',
  'BEGIN_OPENCLAW_INTERNAL_CONTEXT',
  'END_OPENCLAW_INTERNAL_CONTEXT',
  'UNTRUSTED_CHILD_RESULT_DELIMITER'
];

function containsForbiddenRuntimeContextMarker(raw = '') {
  const text = String(raw);
  return FORBIDDEN_RUNTIME_CONTEXT_MARKERS.some((marker) => text.includes(marker));
}

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

const mod = require('../../memory_federation_plane.ts');

assert.deepStrictEqual(
  mod.mapArgs(['memory_federation_plane', 'push', '--device-id=a', '--entries-json=[]']),
  ['sync', '--device-id=a', '--entries-json=[]']
);
assert.deepStrictEqual(
  mod.mapArgs(['download', '--limit=3']),
  ['pull', '--limit=3']
);
assert.deepStrictEqual(mod.mapArgs([]), ['status']);

const out = mod.ensureMutationReceipt({ payload: { ok: true, type: 'memory_federation_plane_sync' } }, 'sync');
assert.ok(typeof out.payload.receipt_hash === 'string');
assert.equal(out.payload.receipt_hash.length >= 64, true);
assert.equal(containsForbiddenRuntimeContextMarker(JSON.stringify(out.payload)), false);

console.log('memory_federation_plane wrapper checks passed');
