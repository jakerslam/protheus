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
const path = require('node:path');
const { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');
const Module = require('node:module');

const ROOT = path.resolve(__dirname, '../..');
const TARGET = path.join(ROOT, 'client', 'runtime', 'lib', 'rust_lane_bridge.ts');

function clearTarget() {
  delete require.cache[require.resolve(TARGET)];
}

function main() {
  const originalLoad = Module._load;
  let spawnCalls = [];

  Module._load = function patchedLoad(request, parent, isMain) {
    if (request === 'child_process') {
      const real = originalLoad.apply(this, arguments);
      return {
        ...real,
        spawnSync(command, args, options) {
          spawnCalls.push({ command, args, cwd: options && options.cwd });
          if (spawnCalls.length === 1) {
            return {
              error: Object.assign(new Error('spawnSync target/debug/infring-ops ENOENT'), {
                code: 'ENOENT'
              }),
              status: null,
              stdout: '',
              stderr: ''
            };
          }
          return {
            error: null,
            status: 0,
            stdout: `${JSON.stringify({ ok: true, type: 'cargo_retry_success' })}\n`,
            stderr: ''
          };
        }
      };
    }
    return originalLoad.apply(this, arguments);
  };

  try {
    process.env.INFRING_OPS_USE_PREBUILT = '1';
    process.env.INFRING_OPS_PREFER_CARGO = '0';
    process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK = '1';
    clearTarget();
    const bridgeModule = require(TARGET);
    const bridge = bridgeModule.createOpsLaneBridge(
      path.join(ROOT, 'client', 'runtime', 'lib'),
      'rust_lane_bridge_fallback_test',
      'directive-kernel',
      { preferLocalCore: true }
    );

    const result = bridge.run(['status']);
    assertNoPlaceholderOrPromptLeak(result, 'rust_lane_bridge_fallback_test');
    assertStableToolingEnvelope(result.payload, 'rust_lane_bridge_fallback_test');
    assert.equal(result.status, 0);
    assert.equal(result.payload.type, 'cargo_retry_success');
    assert.equal(result.fallback_reason, 'stale_prebuilt_retry');
    assert.equal(spawnCalls.length, 2);
    assert.notEqual(spawnCalls[0].command, 'cargo');
    assert.equal(spawnCalls[1].command, 'cargo');
  } finally {
    Module._load = originalLoad;
    clearTarget();
    delete process.env.INFRING_OPS_USE_PREBUILT;
    delete process.env.INFRING_OPS_PREFER_CARGO;
    delete process.env.INFRING_OPS_ALLOW_PROCESS_FALLBACK;
  }

  console.log(JSON.stringify({ ok: true, type: 'rust_lane_bridge_fallback_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
