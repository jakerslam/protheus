#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const transpiled = ts.transpileModule(source, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(transpiled, filename);
  };
}

function run() {
  const mod = require('../../client/runtime/lib/security_plane_bridge.ts');

  assert.equal(mod.BRIDGE_PATH, 'client/runtime/lib/security_plane_bridge.ts');
  assert.equal(mod.LANE_ID, 'security-plane');
  assert.equal(mod.normalizeTool('  LLM_Gateway  '), 'llm_gateway');
  assert.deepEqual(mod.normalizeArgs(['--strict=1', 7, true]), ['--strict=1', '7', 'true']);

  console.log(JSON.stringify({ ok: true, type: 'security_plane_bridge_metadata_test' }));
}

run();
