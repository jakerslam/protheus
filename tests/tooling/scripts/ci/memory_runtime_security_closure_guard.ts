#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (memory-runtime security closure guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/memory_runtime_security_closure_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const violations = [];
for (const check of policy.required_checks || []) {
  const full = path.join(root, check.path);
  const src = fs.existsSync(full) ? fs.readFileSync(full, 'utf8') : '';
  if (!src) violations.push({ id: check.id, path: check.path, reason: 'missing_file' });
  if (check.must_contain && !src.includes(check.must_contain)) violations.push({ id: check.id, path: check.path, reason: 'missing_required_token', token: check.must_contain });
  for (const token of check.must_contain_all || []) {
    if (!src.includes(token)) violations.push({ id: check.id, path: check.path, reason: 'missing_required_token', token });
  }
  if (check.must_not_contain && src.includes(check.must_not_contain)) violations.push({ id: check.id, path: check.path, reason: 'forbidden_token_present', token: check.must_not_contain });
  for (const token of check.must_not_contain_all || []) {
    if (src.includes(token)) violations.push({ id: check.id, path: check.path, reason: 'forbidden_token_present', token });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'memory_runtime_security_closure_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
console.log(JSON.stringify(payload, null, 2));
if (violations.length) process.exit(1);
