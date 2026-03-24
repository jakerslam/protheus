#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
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
const mod = require(path.join(ROOT, 'client/runtime/lib/policy_runtime.ts'));
const merged = mod.deepMerge({ a: 1, rows: [1], obj: { x: 1 } }, { rows: [2], obj: { y: 2 } });
assert.deepEqual(merged, { a: 1, rows: [2], obj: { x: 1, y: 2 } });
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'policy-runtime-'));
const policyPath = path.join(dir, 'policy.json');
fs.writeFileSync(policyPath, JSON.stringify({ obj: { y: 2 }, flag: true }, null, 2));
const runtime = mod.loadPolicyRuntime({ policyPath, defaults: { obj: { x: 1 }, rows: [1] } });
assert.deepEqual(runtime.merged, { obj: { x: 1, y: 2 }, rows: [1], flag: true });
assert.equal(path.isAbsolute(mod.resolvePolicyPath(policyPath)), true);
console.log(JSON.stringify({ ok: true, type: 'policy_runtime_rust_bridge_test' }));
