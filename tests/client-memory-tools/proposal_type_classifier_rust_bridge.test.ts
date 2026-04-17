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
const mod = require(path.join(ROOT, 'client/lib/proposal_type_classifier.ts'));
assert.equal(mod.normalizeTypeKey(' Big Bet! '), 'big_bet');
assert.equal(mod.extractSourceEyeId({ evidence: [{ evidence_ref: 'node|eye:directive_pulse|v1' }] }), 'directive_pulse');
const classification = mod.classifyProposalType({ summary: 'campaign roadmap sequencing' });
assert.equal(classification.type, 'strategy');
assert.equal(classification.inferred, true);
assertNoPlaceholderOrPromptLeak(classification, 'proposal_type_classifier_rust_bridge_test');
assertStableToolingEnvelope({ status: 'ok', classification }, 'proposal_type_classifier_rust_bridge_test');
console.log(JSON.stringify({ ok: true, type: 'proposal_type_classifier_rust_bridge_test' }));
