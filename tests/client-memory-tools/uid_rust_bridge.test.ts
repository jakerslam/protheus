#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const path = require('node:path');
const ts = require('typescript');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

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

const runtimeUid = require(path.join(ROOT, 'client/runtime/lib/uid.ts'));
const adaptiveUid = require(path.join(ROOT, 'core/layer1/memory_runtime/adaptive/uid.ts'));

const first = runtimeUid.stableUid('alpha-seed', { prefix: 'AB', length: 20 });
const second = adaptiveUid.stableUid('alpha-seed', { prefix: 'AB', length: 20 });
assert.equal(first, second);
assert.equal(runtimeUid.isAlnum(first), true);
const random = runtimeUid.randomUid({ prefix: 'xy', length: 18 });
assert.equal(random.length, 18);
assert.equal(runtimeUid.isAlnum(random), true);

assertNoPlaceholderOrPromptLeak({ first, second, random }, 'uid_rust_bridge_test');
assertStableToolingEnvelope({ first, second, random }, 'uid_rust_bridge_test');
console.log(JSON.stringify({ ok: true, type: 'uid_rust_bridge_test' }));
