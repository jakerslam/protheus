#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
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

const ROOT = path.resolve(__dirname, '../..');
const mod = require(path.join(ROOT, 'client/lib/trainability_matrix.ts'));
const policy = mod.defaultPolicy();
assert.equal(policy.provider_rules.internal.allow, true);
const deny = mod.evaluateTrainingDatumTrainability({
  source: { provider: 'external' },
  license: { id: 'mit' },
  consent: { status: 'granted', mode: 'explicit_opt_in' }
});
assert.equal(deny.allow, false);
assert.equal(deny.reason, 'unknown_provider_default_deny');
const allow = mod.evaluateTrainingDatumTrainability({
  source: { provider: 'internal' },
  license: { id: 'internal_protheus' },
  consent: { status: 'granted', mode: 'operator_policy' }
});
assert.equal(allow.allow, true);
assertNoPlaceholderOrPromptLeak({ policy, deny, allow }, 'trainability_matrix_rust_bridge_test');\nassertStableToolingEnvelope({ policy, deny, allow }, 'trainability_matrix_rust_bridge_test');\nconsole.log(JSON.stringify({ ok: true, type: 'trainability_matrix_rust_bridge_test' }));
