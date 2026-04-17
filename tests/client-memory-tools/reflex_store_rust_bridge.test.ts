#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
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
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'reflex-store-rust-'));
  const runtimeRoot = path.join(workspace, 'client', 'runtime');
  fs.mkdirSync(runtimeRoot, { recursive: true });
  process.env.PROTHEUS_WORKSPACE_ROOT = workspace;
  process.env.PROTHEUS_RUNTIME_ROOT = runtimeRoot;
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'core/layer1/memory_runtime/adaptive/reflex_store.ts'));
  const state = mod.ensureReflexState();
  assert.equal(state.policy.max_cells, 2);

  const mutated = mod.mutateReflexState(null, (current) => {
    current.routines.push({
      key: 'Queue Spike',
      name: 'Queue Spike',
      trigger: 'queue_depth>5',
      action: 'scale_up'
    });
    return current;
  });
  assert.equal(mutated.routines.length, 1);
  assert.equal(mutated.routines[0].key, 'queue_spike');
  assert.equal(mutated.routines[0].uid.length > 0, true);

  const reread = mod.readReflexState();
  assert.equal(reread.routines[0].action, 'scale_up');
  assert.equal(fs.existsSync(path.join(runtimeRoot, 'local', 'state', 'security', 'adaptive_mutations.jsonl')), true);
  assert.equal(fs.existsSync(path.join(runtimeRoot, 'local', 'state', 'memory', 'adaptive_pointer_index.json')), true);

  assertNoPlaceholderOrPromptLeak({ state, mutated, reread }, 'reflex_store_rust_bridge_test');\n  assertStableToolingEnvelope(reread, 'reflex_store_rust_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'reflex_store_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
