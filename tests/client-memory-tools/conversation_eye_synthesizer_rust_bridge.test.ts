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
const mod = require(path.join(ROOT, 'client/runtime/systems/sensory/conversation_eye_synthesizer.ts'));
const envelope = mod.synthesizeEnvelope({ message: 'hello world', severity: 'high', tags: ['urgent'] });
assert.equal(envelope.level, 1);
assert.equal(envelope.level_token, 'jot1');
assert.equal(envelope.node_kind, 'insight');
assert.equal(envelope.node_tags.includes('urgent'), true);
assert.match(envelope.node_id, /^conversation-eye-/);
console.log(JSON.stringify({ ok: true, type: 'conversation_eye_synthesizer_rust_bridge_test' }));
