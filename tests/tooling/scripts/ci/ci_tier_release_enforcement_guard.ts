#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/release_gates (CI tier release enforcement guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/release_gates/policies/ci_tier_release_enforcement_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const workflowPath = path.join(root, policy.workflow_path);
const workflow = fs.existsSync(workflowPath) ? fs.readFileSync(workflowPath, 'utf8') : '';
const violations = [];
if (!workflow) violations.push({ kind: 'ci_tier_enforcement_workflow_missing', path: policy.workflow_path });
for (const token of policy.required_tokens || []) {
  if (!workflow.includes(token)) violations.push({ kind: 'ci_tier_enforcement_token_missing', path: policy.workflow_path, token });
}
for (const p of [policy.required_guard, policy.required_manifest, policy.required_policy]) {
  if (!fs.existsSync(path.join(root, p))) violations.push({ kind: 'ci_tier_required_file_missing', path: p });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'ci_tier_release_enforcement_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/ci_tier_release_enforcement_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
