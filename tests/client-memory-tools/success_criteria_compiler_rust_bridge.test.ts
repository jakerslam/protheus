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

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client', 'runtime', 'lib', 'success_criteria_compiler.ts'));

  const proposalRows = mod.compileProposalSuccessCriteria({
    success_criteria: [
      { metric: 'reply_or_interview_count', target: '>= 1 interview signal' },
      { metric: 'reply_or_interview_count', target: '>= 1 interview signal' }
    ],
    action_spec: {
      verify: [{ metric: 'latency', target: 'under 5 s' }]
    }
  }, {
    capability_key: 'proposal:maintenance_patch'
  });
  assert.equal(proposalRows.length, 2);
  assert.equal(proposalRows[0].metric, 'artifact_count');
  assert.equal(proposalRows[1].metric, 'duration_ms');

  const actionRows = mod.toActionSpecRows(proposalRows);
  assert.equal(actionRows.length, 2);
  assert.equal(actionRows[0].metric, 'artifact_count');
  assert.equal(actionRows[1].target, 'duration <=5000ms');

  const compiledRows = mod.compileSuccessCriteriaRows([
    { metric: 'token usage', target: 'at most 1.2k tokens' }
  ], { source: 'validation' });
  assert.equal(compiledRows[0].source, 'validation');
  assert.equal(compiledRows[0].target, 'tokens <=1200');

  assertNoPlaceholderOrPromptLeak({ proposalRows, actionRows, compiledRows }, 'success_criteria_compiler_rust_bridge_test');\n  assertStableToolingEnvelope({ proposalRows, actionRows, compiledRows }, 'success_criteria_compiler_rust_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'success_criteria_compiler_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
