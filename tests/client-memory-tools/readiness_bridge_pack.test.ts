#!/usr/bin/env node
'use strict';

const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');\nconst { assertNoPlaceholderOrPromptLeak, assertStableToolingEnvelope } = require('./runtime_output_guard.ts');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function captureMain(mainFn, argv) {
  const originalWrite = process.stdout.write.bind(process.stdout);
  let output = '';
  process.stdout.write = (chunk, encoding, cb) => {
    output += String(chunk == null ? '' : chunk);
    if (typeof encoding === 'function') encoding();
    if (typeof cb === 'function') cb();
    return true;
  };
  try {
    const code = mainFn(argv);
    return { code, payload: JSON.parse(output.trim()) };
  } finally {
    process.stdout.write = originalWrite;
  }
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'readiness-pack-rust-'));
  const policyPath = path.join(workspace, 'client', 'runtime', 'config', 'readiness_bridge_pack_policy.json');
  fs.mkdirSync(path.dirname(policyPath), { recursive: true });
  fs.writeFileSync(policyPath, JSON.stringify({
    enabled: true,
    strict_default: true,
    latest_path: 'local/state/ops/readiness_bridge_pack/latest.json',
    receipts_path: 'local/state/ops/readiness_bridge_pack/receipts.jsonl'
  }, null, 2));

  process.env.INFRING_WORKSPACE = workspace;
  process.env.INFRING_OPS_USE_PREBUILT = '0';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/runtime/systems/ops/readiness_bridge_pack.ts'));
  const run = captureMain(mod.main, ['run', `--policy=${policyPath}`, '--strict=0']);
  assert.ok([0, 2].includes(run.code));
  assert.equal(run.payload.type, 'readiness_bridge_pack_kernel');
  assert.equal(run.payload.payload.checks.length, 3);

  const status = captureMain(mod.main, ['status', `--policy=${policyPath}`]);
  assert.equal(status.code, 0);
  assert.equal(status.payload.type, 'readiness_bridge_pack_kernel');

  assertNoPlaceholderOrPromptLeak({ run, status }, 'readiness_bridge_pack_rust_bridge_test');\n  assertStableToolingEnvelope(run.payload, 'readiness_bridge_pack_rust_bridge_test');\n  const latestPath = path.join(workspace, 'local', 'state', 'ops', 'readiness_bridge_pack', 'latest.json');
  assert.equal(fs.existsSync(latestPath), true);

  console.log(JSON.stringify({ ok: true, type: 'readiness_bridge_pack_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
