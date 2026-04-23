#!/usr/bin/env node
'use strict';

const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = require('fs').readFileSync(filename, 'utf8');
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

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function main() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'symbiosis-coherence-rust-'));
  const policyPath = path.join(tempRoot, 'client', 'runtime', 'config', 'symbiosis_coherence_policy.json');
  const latestPath = path.join(tempRoot, 'local', 'state', 'symbiosis', 'coherence', 'latest.json');
  const statePath = path.join(tempRoot, 'local', 'state', 'symbiosis', 'coherence', 'state.json');
  const receiptsPath = path.join(tempRoot, 'local', 'state', 'symbiosis', 'coherence', 'receipts.jsonl');

  writeJson(policyPath, {
    version: '1.0',
    shadow_only: true,
    stale_after_minutes: 60,
    paths: {
      state_path: statePath,
      latest_path: latestPath,
      receipts_path: receiptsPath,
      identity_latest_path: path.join(tempRoot, 'local', 'state', 'autonomy', 'identity_anchor', 'latest.json'),
      pre_neuralink_state_path: path.join(tempRoot, 'local', 'state', 'symbiosis', 'pre_neuralink_interface', 'state.json'),
      deep_symbiosis_state_path: path.join(tempRoot, 'local', 'state', 'symbiosis', 'deep_understanding', 'state.json'),
      observer_mirror_latest_path: path.join(tempRoot, 'local', 'state', 'autonomy', 'observer_mirror', 'latest.json')
    }
  });
  writeJson(path.join(tempRoot, 'local', 'state', 'autonomy', 'identity_anchor', 'latest.json'), {
    summary: { identity_drift_score: 0.14, max_identity_drift_score: 0.58, blocked: 0, checked: 12 }
  });
  writeJson(path.join(tempRoot, 'local', 'state', 'symbiosis', 'pre_neuralink_interface', 'state.json'), {
    consent_state: 'granted',
    signals_total: 18,
    routed_total: 16,
    blocked_total: 1
  });
  writeJson(path.join(tempRoot, 'local', 'state', 'symbiosis', 'deep_understanding', 'state.json'), {
    samples: 72,
    style: { directness: 0.92, brevity: 0.81, proactive_delta: 0.88 }
  });
  writeJson(path.join(tempRoot, 'local', 'state', 'autonomy', 'observer_mirror', 'latest.json'), {
    observer: { mood: 'stable' },
    summary: { rates: { ship_rate: 0.78, hold_rate: 0.12 } }
  });

  process.env.SYMBIOSIS_COHERENCE_POLICY_PATH = policyPath;
  process.env.INFRING_OPS_USE_PREBUILT = '0';

  const mod = resetModule(path.join(ROOT, 'client/lib/symbiosis_coherence_signal.ts'));

  const policy = mod.loadSymbiosisCoherencePolicy();
  assert.strictEqual(policy.version, '1.0');

  const evaluated = mod.evaluateSymbiosisCoherenceSignal({ policy_path: policyPath, persist: true });
  assert.strictEqual(evaluated.ok, true);
  assert.strictEqual(evaluated.available, true);
  assert.ok(evaluated.coherence_score > 0.7);
  assert.ok(fs.existsSync(latestPath));
  assert.ok(fs.existsSync(statePath));
  assert.ok(fs.existsSync(receiptsPath));

  const loaded = mod.loadSymbiosisCoherenceSignal({ policy_path: policyPath, refresh: false });
  assert.strictEqual(loaded.available, true);
  assert.ok(String(loaded.latest_path_rel || '').includes('local/state/symbiosis/coherence/latest.json'));

  const recursion = mod.evaluateRecursionRequest({ policy_path: policyPath, requested_depth: 5, shadow_only_override: false });
  assert.strictEqual(recursion.available, true);
  assert.strictEqual(typeof recursion.blocked, 'boolean');
  assert.ok(recursion.allowed_depth == null || recursion.allowed_depth >= 1);

  assertNoPlaceholderOrPromptLeak({ policy, evaluated, loaded, recursion }, 'symbiosis_coherence_rust_bridge_test');
  assertStableToolingEnvelope(evaluated, 'symbiosis_coherence_rust_bridge_test');
  console.log(JSON.stringify({ ok: true, type: 'symbiosis_coherence_rust_bridge_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
