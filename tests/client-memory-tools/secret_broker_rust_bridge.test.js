#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');

const ROOT = path.resolve(__dirname, '../..');

function resetModule(modulePath) {
  delete require.cache[require.resolve(modulePath)];
  return require(modulePath);
}

function main() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'secret-broker-rust-'));
  const runtimeRoot = path.join(tempRoot, 'client', 'runtime');
  const configDir = path.join(runtimeRoot, 'config');
  const stateDir = path.join(runtimeRoot, 'local', 'state', 'security');
  const secretDir = path.join(tempRoot, '.secrets');
  fs.mkdirSync(configDir, { recursive: true });
  fs.mkdirSync(stateDir, { recursive: true });
  fs.mkdirSync(secretDir, { recursive: true });

  const policyPath = path.join(configDir, 'secret_broker_policy.json');
  const statePath = path.join(stateDir, 'secret_broker_state.json');
  const auditPath = path.join(stateDir, 'secret_broker_audit.jsonl');

  fs.writeFileSync(
    policyPath,
    JSON.stringify(
      {
        version: '1.0',
        audit: { include_backend_details: true },
        rotation_policy: {
          warn_after_days: 45,
          max_after_days: 90,
          require_rotated_at: false,
          enforce_on_issue: false
        },
        command_backend: { timeout_ms: 5000 },
        secrets: {
          moltbook_api_key: {
            providers: [
              { type: 'env', env: 'MOLTBOOK_TOKEN', rotated_at_env: 'MOLTBOOK_TOKEN_ROTATED_AT' }
            ],
            rotation: {
              warn_after_days: 30,
              max_after_days: 60,
              require_rotated_at: false,
              enforce_on_issue: false
            }
          },
          command_secret: {
            providers: [
              {
                type: 'command',
                enabled: true,
                command: [
                  process.execPath,
                  '-e',
                  "process.stdout.write(JSON.stringify({value:'cmd-secret',rotated_at:'2026-03-01T00:00:00Z'}))"
                ],
                parse_json: true,
                value_path: 'value',
                rotated_at_path: 'rotated_at'
              }
            ]
          }
        }
      },
      null,
      2
    )
  );

  process.env.SECRET_BROKER_POLICY_PATH = policyPath;
  process.env.SECRET_BROKER_STATE_PATH = statePath;
  process.env.SECRET_BROKER_AUDIT_PATH = auditPath;
  process.env.SECRET_BROKER_SECRETS_DIR = secretDir;
  process.env.SECRET_BROKER_KEY = 'test-secret-broker-key';
  process.env.MOLTBOOK_TOKEN = 'mb-live-secret';
  process.env.MOLTBOOK_TOKEN_ROTATED_AT = '2026-03-01T00:00:00Z';
  process.env.PROTHEUS_OPS_USE_PREBUILT = '0';

  const mod = resetModule(path.join(ROOT, 'client/runtime/lib/secret_broker.ts'));

  const loaded = mod.loadSecretById('moltbook_api_key');
  assert.strictEqual(loaded.ok, true);
  assert.strictEqual(loaded.value, 'mb-live-secret');
  assert.strictEqual(loaded.backend.provider_type, 'env');

  const issued = mod.issueSecretHandle({
    secret_id: 'moltbook_api_key',
    scope: 'skill.moltbook.api',
    caller: 'tests/secret_broker_rust_bridge',
    ttl_sec: 60
  });
  assert.strictEqual(issued.ok, true);
  assert.ok(issued.handle);

  const resolved = mod.resolveSecretHandle(issued.handle, {
    scope: 'skill.moltbook.api',
    caller: 'tests/secret_broker_rust_bridge'
  });
  assert.strictEqual(resolved.ok, true);
  assert.strictEqual(resolved.value, 'mb-live-secret');

  const commandLoaded = mod.loadSecretById('command_secret');
  assert.strictEqual(commandLoaded.ok, true);
  assert.strictEqual(commandLoaded.value, 'cmd-secret');
  assert.strictEqual(commandLoaded.backend.provider_type, 'command');

  const rotation = mod.evaluateSecretRotationHealth({
    secret_ids: ['moltbook_api_key', 'command_secret']
  });
  assert.strictEqual(rotation.ok, true);
  assert.strictEqual(rotation.total, 2);

  const status = mod.secretBrokerStatus();
  assert.strictEqual(status.ok, true);
  assert.strictEqual(status.issued_total, 1);
  assert.strictEqual(status.issued_active, 1);

  const policy = mod.loadPolicy();
  assert.strictEqual(policy.version, '1.0');
  assert.ok(policy.secrets.moltbook_api_key);

  assert.ok(fs.existsSync(statePath));
  assert.ok(fs.existsSync(auditPath));

  console.log(JSON.stringify({ ok: true, type: 'secret_broker_rust_bridge_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
