#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const policyPath = 'validation/release_gates/policies/windows_locked_down_install_replay_policy.json';
const policy = JSON.parse(fs.readFileSync(path.join(ROOT, policyPath), 'utf8'));
const violations: any[] = [];
const readme = fs.readFileSync(path.join(ROOT, 'README.md'), 'utf8');
const ps1 = fs.readFileSync(path.join(ROOT, 'install.ps1'), 'utf8');
const workflow = fs.readFileSync(path.join(ROOT, '.github/workflows/windows-locked-down-install-replay.yml'), 'utf8');
for (const token of policy.required_readme_tokens) if (!readme.includes(token)) violations.push({ kind: 'readme_missing_windows_locked_down_token', token });
for (const token of policy.required_installer_tokens) if (!ps1.includes(token)) violations.push({ kind: 'installer_missing_windows_locked_down_token', token });
for (const token of policy.required_workflow_tokens || []) {
  if (!workflow.includes(token)) violations.push({ kind: 'workflow_missing_windows_locked_down_token', token });
}
const receiptPath = String(policy.required_replay_receipt || '');
if (receiptPath && !workflow.includes(receiptPath)) {
  violations.push({ kind: 'workflow_missing_replay_receipt_path', token: receiptPath });
}
for (const assertion of policy.required_replay_assertions || []) {
  if (!workflow.includes(assertion)) violations.push({ kind: 'workflow_missing_replay_assertion', token: assertion });
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'windows_locked_down_install_replay_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  workflow_path: '.github/workflows/windows-locked-down-install-replay.yml',
  replay_receipt_path: receiptPath || null,
  violations,
};
fs.mkdirSync(path.join(ROOT, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(ROOT, 'core/local/artifacts/windows_locked_down_install_replay_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
