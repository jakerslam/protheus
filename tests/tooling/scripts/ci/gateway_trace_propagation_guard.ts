#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: observability/traces (Gateway trace propagation guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'observability/traces/gateway_trace_propagation_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const sourcePath = path.join(root, policy.source_path);
const source = fs.existsSync(sourcePath) ? fs.readFileSync(sourcePath, 'utf8') : '';
const violations = [];
if (!source) violations.push({ kind: 'gateway_trace_source_missing', path: policy.source_path });
for (const token of policy.required_tokens || []) {
  if (!source.includes(token)) violations.push({ kind: 'gateway_trace_token_missing', path: policy.source_path, token });
}
const traceId = `observability:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'observability', ok: violations.length === 0, type: 'gateway_trace_propagation_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/gateway_trace_propagation_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
