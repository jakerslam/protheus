#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const ts = require('typescript');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

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

const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'catalog-store-runtime-'));
process.env.PROTHEUS_RUNTIME_ROOT = runtimeRoot;
process.env.PROTHEUS_WORKSPACE_ROOT = path.dirname(runtimeRoot);
const ROOT = path.resolve(__dirname, '../..');
const mod = require(path.join(ROOT, 'core/layer1/memory_runtime/adaptive/catalog_store.ts'));
const state = mod.ensureCatalog();
assert.equal(Array.isArray(state.eyes), true);
assert.match(state.eyes[0].uid, /^e/);
const next = mod.mutateCatalog(null, (current) => {
  current.eyes.push({ id: 'test_eye', name: 'Test Eye' });
  return current;
});
assert.equal(next.eyes.some((row) => row.id === 'test_eye'), true);
assertNoPlaceholderOrPromptLeak({ state, next }, 'catalog_store_rust_bridge_test');\nassertStableToolingEnvelope(next, 'catalog_store_rust_bridge_test');\nconsole.log(JSON.stringify({ ok: true, type: 'catalog_store_rust_bridge_test' }));
