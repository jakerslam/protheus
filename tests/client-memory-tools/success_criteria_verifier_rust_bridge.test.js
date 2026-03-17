#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const ts = require('typescript');

const ROOT = path.resolve(__dirname, '..', '..');
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

const {
  parseSuccessCriteriaRows,
  evaluateSuccessCriteria
} = require(path.join(ROOT, 'client', 'runtime', 'lib', 'success_criteria_verifier.ts'));

const proposal = {
  success_criteria: [
    { metric: 'artifact_count', target: '>=1 artifact' },
    { metric: 'reply_or_interview_count', target: '>=1' }
  ],
  action_spec: {
    verify: ['receipt logged']
  }
};

const parsed = parseSuccessCriteriaRows(proposal, { capability_key: 'proposal:internal_patch' });
assert(Array.isArray(parsed), 'expected parsed rows array');
assert(parsed.some((row) => row.metric === 'artifact_count'), 'expected artifact_count row');
assert(parsed.some((row) => row.metric === 'postconditions_ok'), 'expected postconditions_ok row');
assert(!parsed.some((row) => row.metric === 'reply_or_interview_count'), 'expected internal patch remap away from outreach metrics');

const passed = evaluateSuccessCriteria(
  {
    success_criteria: [
      { metric: 'artifact_count', target: '>=1 artifact' },
      { metric: 'token_usage', target: 'tokens <= 500' }
    ]
  },
  {
    exec_ok: true,
    postconditions_ok: true,
    queue_outcome_logged: true,
    dod_diff: {
      artifacts_delta: 2,
      entries_delta: 0,
      revenue_actions_delta: 0
    },
    token_usage: {
      effective_tokens: 420
    }
  },
  {
    capability_key: 'proposal:internal_patch',
    required: true,
    min_count: 2
  }
);
assert.strictEqual(passed.passed, true, 'expected passing verification');
assert.strictEqual(passed.passed_count, 2, 'expected both checks to pass');

const blocked = evaluateSuccessCriteria(
  {
    success_criteria: [
      { metric: 'artifact_count', target: '>=1 artifact' },
      { metric: 'token_usage', target: 'tokens <= 500' }
    ]
  },
  {
    exec_ok: true,
    postconditions_ok: true,
    queue_outcome_logged: true,
    dod_diff: {
      artifacts_delta: 2,
      entries_delta: 0,
      revenue_actions_delta: 0
    },
    token_usage: {
      effective_tokens: 900
    }
  },
  {
    capability_key: 'proposal:internal_patch',
    required: true,
    min_count: 2
  }
);
assert.strictEqual(blocked.passed, false, 'expected token overrun to fail closed');
assert.strictEqual(blocked.contract_not_allowed_count, 0, 'expected no contract violations after remap');
assert(/token_limit_check/.test(String(blocked.primary_failure || '')));

console.log('success_criteria_verifier_rust_bridge.test.js: OK');
