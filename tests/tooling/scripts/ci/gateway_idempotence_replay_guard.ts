#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/regression (Gateway idempotence replay guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/regression/fixtures/gateway_idempotence/gateway_idempotence_replay_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const fixturesDir = path.join(root, policy.fixtures_dir);
const source = fs.existsSync(path.join(root, policy.source_path)) ? fs.readFileSync(path.join(root, policy.source_path), 'utf8') : '';
const violations = [];
for (const token of policy.required_source_tokens || []) {
  if (!source.includes(token)) violations.push({ kind: 'gateway_idempotence_source_token_missing', token, path: policy.source_path });
}
const fixtureFiles = fs.existsSync(fixturesDir) ? fs.readdirSync(fixturesDir).filter((name) => name.endsWith('_replay.json')).sort() : [];
if (fixtureFiles.length < 3) violations.push({ kind: 'gateway_idempotence_fixture_count_too_low', count: fixtureFiles.length, min: 3 });
const scenarios = [];
for (const file of fixtureFiles) {
  const fixture = JSON.parse(fs.readFileSync(path.join(fixturesDir, file), 'utf8'));
  scenarios.push(fixture.scenario || file);
  if (!fixture.input || !fixture.expected) violations.push({ kind: 'gateway_idempotence_fixture_shape_invalid', file });
  if (fixture.expected?.must_not_restart && !fixture.expected?.forbidden_output_tokens?.some((token) => String(token).includes('restart'))) {
    violations.push({ kind: 'gateway_idempotence_fixture_missing_restart_forbidden_token', file });
  }
  if (fixture.expected?.must_use_requested_port && !String(fixture.input?.command || '').includes(String(fixture.expected.must_use_requested_port))) {
    violations.push({ kind: 'gateway_idempotence_fixture_requested_port_not_in_command', file });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'gateway_idempotence_replay_guard', generated_at: new Date().toISOString(), policy_path: policyPath, fixture_count: fixtureFiles.length, scenarios, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/gateway_idempotence_replay_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
