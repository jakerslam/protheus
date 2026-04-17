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
process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';
const mod = require(path.join(ROOT, 'client/lib/emergency_stop.ts'));

const backup = fs.existsSync(mod.STOP_PATH) ? fs.readFileSync(mod.STOP_PATH, 'utf8') : null;

try {
  const engaged = mod.engageEmergencyStop({
    scopes: ['routing', 'routing', 'all'],
    approval_note: 'approved for integration test',
    actor: 'tester',
    reason: 'integration'
  });
  assert.equal(engaged.engaged, true);
  assert.deepEqual(mod.normalizeScopes(['routing,autonomy', 'autonomy']), ['autonomy', 'routing']);
  assert.equal(mod.isEmergencyStopEngaged('routing').engaged, true);
  assertNoPlaceholderOrPromptLeak({ engaged }, 'emergency_stop_rust_bridge_test');\n  assertStableToolingEnvelope(engaged, 'emergency_stop_rust_bridge_test');\n  const released = mod.releaseEmergencyStop({
    approval_note: 'approved release integration',
    actor: 'tester',
    reason: 'done'
  });
  assert.equal(released.engaged, false);
  assert.equal(fs.existsSync(mod.STOP_PATH), true);
} finally {
  if (backup == null) {
    if (fs.existsSync(mod.STOP_PATH)) fs.unlinkSync(mod.STOP_PATH);
  } else {
    fs.writeFileSync(mod.STOP_PATH, backup, 'utf8');
  }
}

console.log(JSON.stringify({ ok: true, type: 'emergency_stop_rust_bridge_test' }));
