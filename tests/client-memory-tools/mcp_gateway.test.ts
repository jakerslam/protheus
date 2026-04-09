#!/usr/bin/env node
'use strict';

const assert = require('assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '../..');
const SCRIPT = path.join(ROOT, 'adapters', 'cognition', 'skills', 'mcp', 'mcp_gateway.ts');

function runGateway(args, env = {}) {
  const proc = spawnSync(process.execPath, [SCRIPT, ...args], {
    cwd: ROOT,
    env: { ...process.env, ...env },
    encoding: 'utf8'
  });
  const lines = String(proc.stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  const last = lines.length ? lines[lines.length - 1] : '{}';
  let json = {};
  try {
    json = JSON.parse(last);
  } catch {
    json = { ok: false, error: 'invalid_json', raw: last };
  }
  return { proc, json };
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2));
}

function main() {
  const tempRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'protheus-mcp-gateway-test-'));
  const stateRoot = path.join(tempRoot, 'local', 'state', 'client', 'cognition', 'skills', 'mcp_gateway');
  const policyPath = path.join(tempRoot, 'policy.json');
  const registryPath = path.join(tempRoot, 'registry.json');
  const installsPath = path.join(stateRoot, 'installs.json');
  const latestPath = path.join(stateRoot, 'latest.json');
  const eventsPath = path.join(stateRoot, 'events.jsonl');
  const receiptsPath = path.join(stateRoot, 'receipts.jsonl');
  const memoryDir = path.join(tempRoot, 'memory');
  const adaptiveIndexPath = path.join(tempRoot, 'adaptive', 'index.json');

  writeJson(policyPath, {
    version: '1.0',
    enabled: true,
    strict_default: true,
    event_stream: { enabled: true, publish: true, stream: 'skills.mcp_gateway.test' },
    risk: { default_tier: 2, require_explicit_approval_tier: 3 },
    paths: {
      registry_path: registryPath,
      installs_path: installsPath,
      latest_path: latestPath,
      events_path: eventsPath,
      receipts_path: receiptsPath,
      memory_dir: memoryDir,
      adaptive_index_path: adaptiveIndexPath
    }
  });

  writeJson(registryPath, {
    skills: [
      { id: 'filesystem_agent', title: 'Filesystem Agent', source: 'mcp://filesystem', trust_tier: 'verified' },
      { id: 'issue_triage', title: 'Issue Triage', source: 'mcp://issues', trust_tier: 'standard' }
    ]
  });

  const status = runGateway(['status', `--policy=${policyPath}`, '--apply=0', '--strict=1']);
  assert.strictEqual(status.proc.status, 0, `status failed: ${status.proc.stderr}`);
  assert.strictEqual(status.json.ok, true);
  assert.strictEqual(status.json.details.registry_count, 2);

  const discover = runGateway(['discover', `--policy=${policyPath}`, '--apply=0', '--strict=1']);
  assert.strictEqual(discover.proc.status, 0, `discover failed: ${discover.proc.stderr}`);
  assert.strictEqual(discover.json.ok, true);
  assert.ok(Array.isArray(discover.json.details.capability_matrix));
  assert.strictEqual(discover.json.details.capability_matrix.length, 2);

  const install = runGateway(['install', '--id=filesystem_agent', `--policy=${policyPath}`, '--apply=1', '--strict=1']);
  assert.strictEqual(install.proc.status, 0, `install failed: ${install.proc.stderr}`);
  assert.strictEqual(install.json.ok, true);
  assert.strictEqual(install.json.details.id, 'filesystem_agent');
  assert.ok(fs.existsSync(installsPath), 'installs state missing after install');

  const statusAfterInstall = runGateway(['status', `--policy=${policyPath}`, '--apply=0', '--strict=1']);
  assert.strictEqual(statusAfterInstall.proc.status, 0);
  assert.strictEqual(statusAfterInstall.json.details.installed_count, 1);

  const uninstall = runGateway(['uninstall', '--id=filesystem_agent', `--policy=${policyPath}`, '--apply=1', '--strict=1']);
  assert.strictEqual(uninstall.proc.status, 0, `uninstall failed: ${uninstall.proc.stderr}`);
  assert.strictEqual(uninstall.json.ok, true);
  assert.strictEqual(uninstall.json.details.removed, true);

  const exportResult = runGateway(['export', `--policy=${policyPath}`, '--apply=0', '--strict=1']);
  assert.strictEqual(exportResult.proc.status, 0, `export failed: ${exportResult.proc.stderr}`);
  assert.strictEqual(exportResult.json.ok, true);
  assert.strictEqual(exportResult.json.details.registry_count, 2);
  assert.ok(Array.isArray(exportResult.json.details.capability_matrix));

  const bypass = runGateway(['status', `--policy=${policyPath}`, '--bypass=1', '--strict=1']);
  assert.strictEqual(bypass.proc.status, 2, `expected bypass rejection exit 2, got ${bypass.proc.status}`);
  assert.strictEqual(bypass.json.ok, false);
  assert.strictEqual(bypass.json.error, 'bypass_forbidden');

  assert.ok(fs.existsSync(latestPath), 'latest receipt missing');
  assert.ok(fs.existsSync(eventsPath), 'events receipt log missing');
  assert.ok(fs.existsSync(receiptsPath), 'receipts log missing');
  assert.ok(fs.existsSync(adaptiveIndexPath), 'adaptive index missing');

  console.log(JSON.stringify({
    ok: true,
    type: 'mcp_gateway_runtime_test',
    status: 'pass'
  }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
