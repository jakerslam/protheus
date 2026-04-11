#!/usr/bin/env node
'use strict';

const assert = require('node:assert');
const fs = require('node:fs');
const path = require('node:path');
const { spawnSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.join(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const CLIENT_WRAPPER = path.join(ROOT, 'client/runtime/systems/personas/orchestration.ts');
const POLICY_PATH = path.join(
  ROOT,
  'client/cognition/personas/organization/shadow_deployment_policy.json',
);

function parseJsonOutput(text) {
  const trimmed = String(text || '').trim();
  if (!trimmed) return null;
  try {
    return JSON.parse(trimmed);
  } catch {}
  const lines = trimmed.split('\n');
  for (let index = 0; index < lines.length; index += 1) {
    const candidate = lines.slice(index).join('\n').trim();
    if (!candidate.startsWith('{') || !candidate.endsWith('}')) continue;
    try {
      return JSON.parse(candidate);
    } catch {}
  }
  return null;
}

function runScript(args) {
  const proc = spawnSync(process.execPath, [ENTRYPOINT, CLIENT_WRAPPER, ...args], {
    cwd: ROOT,
    encoding: 'utf8',
  });
  const payload = parseJsonOutput(proc.stdout) || parseJsonOutput(proc.stderr);
  assert.strictEqual(proc.status, 0, proc.stderr || proc.stdout || `failed:${args.join(' ')}`);
  assert(payload && payload.ok === true, `expected ok payload for ${args.join(' ')}`);
  assert(payload.payload && payload.payload.ok === true, 'expected conduit payload');
  assert(payload.payload.payload && payload.payload.payload.ok === true, 'expected runtime payload');
  return payload.payload.payload;
}

function main() {
  const policy = JSON.parse(fs.readFileSync(POLICY_PATH, 'utf8'));

  assert.strictEqual(policy.version, '1.0.0');
  assert.strictEqual(policy.enabled, true);
  assert.strictEqual(policy.feature_flags.meeting, true);
  assert.strictEqual(policy.feature_flags.project, true);
  assert.strictEqual(policy.feature_flags.telemetry, true);
  assert.strictEqual(typeof policy.kill_switch.enabled, 'boolean');
  assert.strictEqual(typeof policy.kill_switch.reason, 'string');
  assert.strictEqual(policy.resource_isolation.enforce, true);
  assert.strictEqual(policy.resource_isolation.max_concurrent_meetings, 2);
  assert.strictEqual(policy.resource_isolation.max_concurrent_projects, 2);
  assert(policy.resource_isolation.max_estimated_tokens > 0);
  assert(policy.resource_isolation.max_estimated_runtime_ms > 0);

  const meeting = runScript(['meeting', 'Policy contract check', '--dry-run=1', '--strict=1']);
  const telemetry = runScript(['telemetry', '--dry-run=1']);

  assert.strictEqual(meeting.system_id, 'SYSTEMS-PERSONAS-ORCHESTRATION');
  assert.strictEqual(meeting.command, 'meeting');
  assert.strictEqual(meeting.strict, true);
  assert.strictEqual(telemetry.command, 'telemetry');
  assert.strictEqual(telemetry.system_id, 'SYSTEMS-PERSONAS-ORCHESTRATION');
  assert.strictEqual(meeting.latest_path, telemetry.latest_path);
  assert.strictEqual(meeting.history_path, telemetry.history_path);
  assert(
    Array.isArray(telemetry.claim_evidence)
      && telemetry.claim_evidence.some((row) => row && row.id === 'runtime_system_mutation_receipted'),
    'expected telemetry path to remain receipted and fail-closed',
  );
  assert.strictEqual(fs.existsSync(path.join(ROOT, telemetry.latest_path)), true);
  assert.strictEqual(fs.existsSync(path.join(ROOT, telemetry.history_path)), true);

  console.log(JSON.stringify({ ok: true, type: 'personas_orchestration_policies_test' }));
}

if (require.main === module) {
  main();
}

module.exports = { main };
