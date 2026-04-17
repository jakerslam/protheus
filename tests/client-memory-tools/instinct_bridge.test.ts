#!/usr/bin/env node
'use strict';

// SRS coverage: V10-ULTIMATE-002.1, V10-ULTIMATE-002.2, V10-ULTIMATE-002.3

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
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

const bridge = require('../../client/runtime/lib/instinct_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'instinct-bridge-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');
  const lineagePath = path.join(tmpDir, 'lineage.jsonl');

  const model = bridge.coldStartModel({
    tools: ['search', 'shell', 'memory'],
    skills: ['planner', 'summarizer'],
    adapters: ['receipt-provenance', 'mcp'],
    modes: ['swarm', 'pure', 'rich'],
    memory_lanes: ['recent', 'semantic'],
    platform: 'desktop',
    memory_mb: 16384,
    cpu_cores: 8,
    battery_pct: 90,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(model.self_model.strongest_profile, 'rich');
  assert.strictEqual(model.self_model.tools.length, 3);

  const activation = bridge.activate({
    requested_capabilities: ['swarm', 'memory', 'provenance'],
    event: 'cold_start',
    battery_pct: 18,
    low_power: true,
    network_available: false,
    state_path: statePath,
    history_path: historyPath,
  });
  assert.strictEqual(activation.activation.selected_profile, 'tiny-max');
  assert.strictEqual(activation.activation.activated_capabilities.includes('memory'), true);
  assert.strictEqual(
    activation.activation.rejected_capabilities.some((row) => row.capability === 'swarm'),
    true
  );

  const refinement = bridge.refine({
    evidence: [
      { dimension: 'memory', success: true, latency_ms: 120, blob_ref: 'blob://memory-ok' },
      { dimension: 'swarm', success: false, latency_ms: 1900, blob_ref: 'blob://swarm-bad' }
    ],
    state_path: statePath,
    history_path: historyPath,
    lineage_path: lineagePath,
  });
  assert.strictEqual(refinement.refinement.rollbackable, true);
  assert.strictEqual(fs.existsSync(lineagePath), true);

  const status = bridge.status({ state_path: statePath, history_path: historyPath });
  assert.strictEqual(status.activations, 1);
  assert.strictEqual(status.refinements, 1);
  assert.strictEqual(status.models, 2);

  assertNoPlaceholderOrPromptLeak({ model, activation, refinement, status }, 'instinct_bridge_test');\n  assertStableToolingEnvelope(status, 'instinct_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'instinct_bridge_test' }));
}

run();
