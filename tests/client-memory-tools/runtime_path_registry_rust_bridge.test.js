#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
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

const ROOT = path.resolve(__dirname, '../..');
process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

const mod = require(path.join(ROOT, 'client/runtime/lib/runtime_path_registry.ts'));
assert.equal(mod.CANONICAL_PATHS.client_state_root, 'client/runtime/local/state');
assert.equal(mod.normalizeForRoot('/repo/client/runtime', 'client/runtime/local/state'), 'local/state');
assert.equal(mod.resolveCanonical('/repo/client/runtime', 'client/runtime/local/state'), '/repo/client/runtime/local/state');
assert.equal(mod.resolveClientState('/repo', 'security/a.json'), '/repo/client/runtime/local/state/security/a.json');
assert.equal(mod.resolveCoreState('/repo', 'ops/b.json'), '/repo/core/local/state/ops/b.json');

console.log(JSON.stringify({ ok: true, type: 'runtime_path_registry_rust_bridge_test' }));
