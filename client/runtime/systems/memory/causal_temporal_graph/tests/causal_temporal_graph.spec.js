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

const mod = require('../../causal_temporal_graph.ts');

assert.deepStrictEqual(
  mod.mapArgs(['causal-temporal-graph', 'build', '--actor=ci']),
  ['record', '--event-id=build-latest', '--summary=legacy_build_alias', '--actor=system', '--apply=0', '--actor=ci']
);

assert.deepStrictEqual(
  mod.mapArgs(['query', '--event-id=e1']),
  ['blame', '--event-id=build-latest', '--event-id=e1']
);

assert.deepStrictEqual(mod.mapArgs([]), ['status']);

const out = mod.ensureMutationReceipt({ payload: { ok: true, type: 'causal_temporal_graph_record' } }, 'record');
assert.ok(typeof out.payload.receipt_hash === 'string');
assert.equal(out.payload.receipt_hash.length >= 64, true);

console.log('causal_temporal_graph wrapper checks passed');
