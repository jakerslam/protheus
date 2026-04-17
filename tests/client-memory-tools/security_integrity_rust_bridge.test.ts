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

function writeFile(root, rel, contents) {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, contents);
}

function main() {
  const workspace = fs.mkdtempSync(path.join(os.tmpdir(), 'security-integrity-rust-'));
  const runtimeRoot = path.join(workspace, 'client', 'runtime');
  fs.mkdirSync(runtimeRoot, { recursive: true });
  writeFile(runtimeRoot, 'systems/security/guard.js', 'module.exports = 1;\n');
  writeFile(runtimeRoot, 'config/directives/policy.yaml', 'mode: strict\n');

  process.env.PROTHEUS_WORKSPACE_ROOT = workspace;
  process.env.PROTHEUS_RUNTIME_ROOT = runtimeRoot;
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
  process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = '120000';

  const mod = resetModule(path.join(ROOT, 'client/runtime/lib/security_integrity.ts'));

  const policy = mod.loadPolicy();
  assert.equal(policy.version, '1.0');

  const files = mod.collectPresentProtectedFiles(policy);
  assert.equal(files.includes('systems/security/guard.js'), true);
  assert.equal(files.includes('config/directives/policy.yaml'), true);

  const sealed = mod.sealIntegrity(undefined, { sealed_by: 'test' });
  assert.equal(sealed.ok, true);
  assert.equal(sealed.sealed_files, 2);

  const verified = mod.verifyIntegrity();
  assert.equal(verified.ok, true);

  writeFile(runtimeRoot, 'systems/security/guard.js', 'module.exports = 2;\n');
  const broken = mod.verifyIntegrity();
  assert.equal(broken.ok, false);
  assert.equal(
    broken.violations.some((row) => row.type === 'hash_mismatch' && row.file === 'systems/security/guard.js'),
    true
  );

  const appended = mod.appendIntegrityEvent({ type: 'hash_mismatch', file: 'systems/security/guard.js' });
  assert.equal(appended.ok, true);
  assert.equal(fs.existsSync(mod.DEFAULT_LOG_PATH), true);

  assertNoPlaceholderOrPromptLeak({ policy, files, sealed, verified, broken, appended }, 'security_integrity_rust_bridge_test');\n  assertStableToolingEnvelope(sealed, 'security_integrity_rust_bridge_test');\n  console.log(JSON.stringify({ ok: true, type: 'security_integrity_rust_bridge_test' }));
}

try {
  main();
} catch (error) {
  console.error(error);
  process.exit(1);
}
