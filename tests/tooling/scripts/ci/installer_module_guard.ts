#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (installer module guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/installer_module_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const violations = [];
for (const mod of policy.modules || []) {
  const full = path.join(root, mod.path);
  if (!fs.existsSync(full)) {
    violations.push({ kind: 'installer_module_missing', path: mod.path });
    continue;
  }
  const source = fs.readFileSync(full, 'utf8');
  for (const token of mod.required_tokens || []) {
    if (!source.includes(token)) violations.push({ kind: 'installer_module_missing_token', path: mod.path, token });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'installer_module_guard', generated_at: new Date().toISOString(), policy_path: policyPath, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/installer_module_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
