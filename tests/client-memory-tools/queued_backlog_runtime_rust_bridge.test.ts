#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
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

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'queued-backlog-rust-'));
  fs.mkdirSync(path.join(workspace, 'client', 'runtime', 'config'), { recursive: true });
  const prevWorkspace = process.env.INFRING_WORKSPACE;
  const prevPrebuilt = process.env.INFRING_OPS_USE_PREBUILT;
  const prevTimeout = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS;
  process.env.INFRING_WORKSPACE = workspace;
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';
  try {
    const mod = resetModule(path.join(ROOT, 'client', 'runtime', 'lib', 'queued_backlog_runtime.ts'));
    const latestPath = mod.resolvePath('local/state/demo/latest.json', 'local/state/demo/latest.json');
    assert.match(latestPath, /client\/runtime\/local\/state\/demo\/latest\.json$/);
    assert.strictEqual(latestPath.includes(workspace), true);

    mod.writeJsonAtomic(latestPath, { ok: true, ts: mod.nowIso() });
    const latest = mod.readJson(latestPath, null);
    assert.equal(latest.ok, true);
    assert.equal(typeof latest.ts, 'string');

    const historyPath = mod.resolvePath('local/state/demo/history.jsonl', 'local/state/demo/history.jsonl');
    if (fs.existsSync(historyPath)) {
      fs.unlinkSync(historyPath);
    }
    mod.appendJsonl(historyPath, { seq: 1 });
    mod.appendJsonl(historyPath, { seq: 2 });
    const rows = mod.readJsonl(historyPath);
    assert.equal(rows.length, 2);
    assert.equal(mod.stableHash({ hello: 'world' }, 12).length, 12);

    const policyPath = path.join(workspace, 'client', 'runtime', 'config', 'lane_policy.json');
    fs.writeFileSync(policyPath, JSON.stringify({ enabled: false, strict_default: false }, null, 2));
    const policy = mod.loadPolicy(policyPath, { version: '1.0', enabled: true });
    assert.equal(policy.enabled, false);
    assert.equal(typeof policy.version, 'string');
    assert.equal(mod.rollingAverage([1, 2, 3]), 2);
    assert.equal(mod.median([1, 9, 3]), 3);

    console.log(JSON.stringify({ ok: true, type: 'queued_backlog_runtime_rust_bridge_test' }));
  } finally {
    if (prevWorkspace == null) delete process.env.INFRING_WORKSPACE;
    else process.env.INFRING_WORKSPACE = prevWorkspace;
    if (prevPrebuilt == null) delete process.env.INFRING_OPS_USE_PREBUILT;
    else process.env.INFRING_OPS_USE_PREBUILT = prevPrebuilt;
    if (prevTimeout == null) delete process.env.INFRING_OPS_LOCAL_TIMEOUT_MS;
    else process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = prevTimeout;
    fs.rmSync(workspace, { recursive: true, force: true });
  }
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
