#!/usr/bin/env node
'use strict';

// SRS coverage: V6-WORKFLOW-008.9

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
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

const bridge = require('../../adapters/polyglot/semantic_kernel_dotnet_bridge.ts');

function run() {
  const tmpDir = fs.mkdtempSync(path.join(os.tmpdir(), 'semantic-kernel-dotnet-'));
  const statePath = path.join(tmpDir, 'state.json');
  const historyPath = path.join(tmpDir, 'history.jsonl');

  const registered = bridge.registerBridge({
    name: 'semantic-kernel-dotnet-parity',
    capabilities: ['plugin', 'agent'],
    state_path: statePath,
    history_path: historyPath,
  });
  const invoked = bridge.invokeBridge({
    bridge_id: registered.bridge.bridge_id,
    operation: 'invoke-plugin',
    dry_run: true,
    args: { plugin: 'faq_router', input: 'hello' },
    state_path: statePath,
    history_path: historyPath,
  });

  assert.strictEqual(invoked.invocation.mode, 'dry_run');
  assert.strictEqual(invoked.invocation.simulated, true);
  console.log(JSON.stringify({ ok: true, type: 'semantic_kernel_dotnet_bridge_test' }));
}

run();
