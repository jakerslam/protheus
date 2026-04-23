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

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/runtime/lib/quorum_validator.ts'));
  const approved = mod.evaluateProposalQuorum({
    type: 'security_policy_update',
    risk: 'high',
    suggested_next_command: 'policy_rootd preview --dry-run --objective=T5_GUARD',
    meta: {
      directive_objective_id: 'T5_GUARD',
      rollback_plan: 'rollback to previous guard profile'
    },
    success_criteria: [
      { metric: 'latency', target: '< 2s' }
    ]
  });
  assert.equal(approved.requires_quorum, true);
  assert.equal(approved.ok, true);
  assert.equal(approved.reason, 'approved');

  const disagreement = mod.evaluateProposalQuorum({
    type: 'strategy_shift',
    risk: 'high',
    suggested_next_command: 'strategy_controller apply',
    meta: {
      directive_objective_id: 'T5_PLAN',
      rollback_plan: 'rollback to previous strategy'
    },
    success_criteria: [
      { metric: 'latency', target: '< 2s' }
    ]
  });
  assert.equal(disagreement.requires_quorum, true);
  assert.equal(disagreement.ok, false);
  assert.equal(disagreement.reason, 'validator_disagreement');

  assertNoPlaceholderOrPromptLeak({ approved, disagreement }, 'quorum_validator_rust_bridge_test');
  assertStableToolingEnvelope(approved, 'quorum_validator_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'quorum_validator_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
