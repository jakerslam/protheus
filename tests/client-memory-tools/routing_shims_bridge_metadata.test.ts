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
  const llm = require('../../client/runtime/systems/routing/llm_gateway_failure_classifier.ts');
  const provider = require('../../client/runtime/systems/routing/provider_onboarding_manifest.ts');

  assert.equal(llm.BRIDGE_PATH, 'client/runtime/systems/routing/llm_gateway_failure_classifier.ts');
  assert.equal(llm.ORCHESTRATION_SCRIPT, 'surface/orchestration/scripts/llm_gateway_failure_classifier.ts');
  assert.equal(llm.MODULE_KEY, 'llm_gateway_failure_classifier');
  assert.deepEqual(llm.normalizeArgs(['--strict', 1, true]), ['--strict', '1', 'true']);

  assert.equal(provider.BRIDGE_PATH, 'client/runtime/systems/routing/provider_onboarding_manifest.ts');
  assert.equal(provider.ORCHESTRATION_SCRIPT, 'surface/orchestration/scripts/provider_onboarding_manifest.ts');
  assert.equal(provider.MODULE_KEY, 'provider_onboarding_manifest');
  assert.deepEqual(provider.normalizeArgs(['--apply', false, 7]), ['--apply', 'false', '7']);

  console.log(JSON.stringify({ ok: true, type: 'routing_shims_bridge_metadata_test' }));
}

run();
