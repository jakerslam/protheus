#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const ts = require('typescript');

if (!require.extensions['.ts']) {
  require.extensions['.ts'] = function compileTs(module, filename) {
    const source = fs.readFileSync(filename, 'utf8');
    const output = ts.transpileModule(source, {
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
    module._compile(output, filename);
  };
}

const mod = require(path.resolve(__dirname, '..', '..', 'client', 'runtime', 'lib', 'ops_domain_conduit_runner.ts'));

function run() {
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';
  process.env.PROTHEUS_OPS_DOMAIN_SKIP_RUNTIME_GATE = '0';
  process.env.PROTHEUS_OPS_DOMAIN_STDIO_TIMEOUT_MS = '34567';
  delete process.env.PROTHEUS_OPS_DOMAIN_BRIDGE_TIMEOUT_MS;
  delete process.env.PROTHEUS_CONDUIT_BRIDGE_TIMEOUT_MS;

  const parsed = mod.parseArgs(['--domain', 'legacy-retired-lane', 'run', '--lane-id=FOO-3']);
  assert.equal(parsed.domain, 'legacy-retired-lane');
  assert.deepStrictEqual(parsed._, ['run']);

  const options = mod.buildRunOptions(parsed);
  assert.equal(options.skipRuntimeGate, false);
  assert.equal(options.stdioTimeoutMs, 34567);
  assert.equal(options.timeoutMs, 125000);
  assert.equal(options.runContext, null);
}

run();
console.log(JSON.stringify({ ok: true, type: 'ops_domain_conduit_runner_rust_bridge_test' }));
