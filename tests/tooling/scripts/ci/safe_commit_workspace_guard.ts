#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (safe commit workspace guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/safe_commit_workspace_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const sourcePath = path.join(root, policy.tool_path);
const source = fs.existsSync(sourcePath) ? fs.readFileSync(sourcePath, 'utf8') : '';
const violations = [];
if (!source) violations.push({ kind: 'safe_commit_tool_missing', path: policy.tool_path });
for (const token of policy.required_tokens || []) {
  if (!source.includes(token)) violations.push({ kind: 'safe_commit_tool_missing_token', token });
}
for (const token of policy.forbidden_tool_tokens || []) {
  if (source.includes(token)) violations.push({ kind: 'safe_commit_tool_forbidden_mutation_token', token });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'safe_commit_workspace_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/safe_commit_workspace_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
