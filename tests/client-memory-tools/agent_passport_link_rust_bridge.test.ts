#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
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
const runtimeRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'passport-link-'));
process.env.PROTHEUS_RUNTIME_ROOT = runtimeRoot;
process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';
const mod = require(path.join(ROOT, 'client/runtime/lib/agent_passport_link.ts'));
const out = mod.linkReceiptToPassport('/tmp/receipt.json', { ok: true, type: 'unit' });
assert.equal(out && out.ok, true);
const chainPath = path.join(runtimeRoot, 'local', 'state', 'security', 'passport_iteration_chain.jsonl');
assert.equal(fs.existsSync(chainPath), true);
const raw = fs.readFileSync(chainPath, 'utf8');
assert.match(raw, /receipt_link/);
assertNoPlaceholderOrPromptLeak({ out, raw }, 'agent_passport_link_rust_bridge_test');
assertStableToolingEnvelope(out, 'agent_passport_link_rust_bridge_test');
console.log(JSON.stringify({ ok: true, type: 'agent_passport_link_rust_bridge_test' }));
