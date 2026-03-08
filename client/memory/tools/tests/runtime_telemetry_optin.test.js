#!/usr/bin/env node
'use strict';

const fs = require('fs');
const os = require('os');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const TS_ENTRY = path.join(ROOT, 'lib', 'ts_entrypoint.js');
const SCRIPT = path.join(ROOT, 'systems', 'observability', 'runtime_telemetry_optin.ts');

function run(args, env = {}) {
  return spawnSync(process.execPath, [TS_ENTRY, SCRIPT, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      ...env
    }
  });
}

function parseJson(raw) {
  try {
    return JSON.parse(String(raw || '').trim());
  } catch {
    return null;
  }
}

function fail(message) {
  console.error(`runtime_telemetry_optin.test.js FAILED: ${message}`);
  process.exit(1);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function writeJson(filePath, payload) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function main() {
  const tempDir = fs.mkdtempSync(path.join(os.tmpdir(), 'runtime-telemetry-optin-'));
  const events = path.join(tempDir, 'events.jsonl');
  const latest = path.join(tempDir, 'latest.json');
  const policy = path.join(tempDir, 'policy.json');
  const envBase = {
    RUNTIME_TELEMETRY_POLICY_PATH: policy,
    PROTHEUS_TELEMETRY_SALT: 'test-salt'
  };

  writeJson(policy, {
    schema_id: 'runtime_telemetry_policy',
    schema_version: '1.0.0',
    enabled: false,
    paths: {
      events_path: events,
      aggregate_path: latest
    },
    privacy: {
      salt_env_var: 'PROTHEUS_TELEMETRY_SALT'
    }
  });

  const disabled = run(['emit-usage', '--counter=cli_run', '--value=2'], envBase);
  assert(disabled.status === 0, `disabled emit should not fail: ${disabled.stderr}`);
  const disabledPayload = parseJson(disabled.stdout);
  assert(disabledPayload && disabledPayload.emitted === false, 'disabled emit should not write telemetry');
  assert(!fs.existsSync(events), 'events file should not exist when telemetry is disabled');

  writeJson(policy, {
    schema_id: 'runtime_telemetry_policy',
    schema_version: '1.0.0',
    enabled: true,
    paths: {
      events_path: events,
      aggregate_path: latest
    },
    privacy: {
      salt_env_var: 'PROTHEUS_TELEMETRY_SALT'
    }
  });

  const usage = run(['emit-usage', '--counter=cli_run', '--value=3'], envBase);
  assert(usage.status === 0, `usage emit should pass: ${usage.stderr}`);
  const usagePayload = parseJson(usage.stdout);
  assert(usagePayload && usagePayload.emitted === true, 'usage emit expected true');

  const crash = run(['emit-crash', '--component=spine', '--reason=timeout'], envBase);
  assert(crash.status === 0, `crash emit should pass: ${crash.stderr}`);

  const perf = run(['emit-perf', '--component=conduit', '--metric=latency_ms', '--value=14.7'], envBase);
  assert(perf.status === 0, `perf emit should pass: ${perf.stderr}`);

  assert(fs.existsSync(events), 'events path should exist after enabled emits');
  assert(fs.existsSync(latest), 'aggregate path should exist after enabled emits');

  const rows = fs.readFileSync(events, 'utf8').trim().split('\n').filter(Boolean).map((line) => JSON.parse(line));
  assert(rows.length === 3, `expected 3 telemetry rows, got ${rows.length}`);
  assert(rows.every((row) => typeof row.host_fingerprint === 'string' && row.host_fingerprint.length > 0), 'host fingerprints must be present');
  assert(rows.every((row) => !String(row.host_fingerprint).includes(os.hostname())), 'host fingerprint must be anonymized');

  const aggregate = JSON.parse(fs.readFileSync(latest, 'utf8'));
  assert(Number(aggregate.usage.cli_run || 0) >= 3, 'usage aggregate should include cli_run counter');
  assert(Number(aggregate.crash_count || 0) >= 1, 'crash aggregate should increment');
  assert(aggregate.perf && aggregate.perf['conduit.latency_ms'], 'perf aggregate should include conduit.latency_ms');

  console.log('runtime_telemetry_optin.test.js: OK');
}

main();
