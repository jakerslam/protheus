#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/regression (disposable Gateway live replay guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/regression/fixtures/gateway_idempotence/gateway_disposable_live_replay_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
const steps = Array.isArray(policy.steps) ? policy.steps : [];

if (!report) violations.push({ kind: 'gateway_disposable_live_replay_report_missing', path: policy.report_path });
if (!steps.some((step) => step.kind === 'cleanup' && step.always_cleanup)) violations.push({ kind: 'gateway_disposable_live_replay_missing_cleanup_step' });
if (!steps.some((step) => step.kind === 'status' && step.mutates === false)) violations.push({ kind: 'gateway_disposable_live_replay_missing_read_only_status_step' });
for (const step of steps) {
  if (step.mutates && step.requires_apply !== true) {
    violations.push({ kind: 'gateway_disposable_live_replay_mutating_step_without_apply_gate', id: step.id });
  }
  if (JSON.stringify(step.command || []).includes('4173')) {
    violations.push({ kind: 'gateway_disposable_live_replay_uses_primary_port', id: step.id });
  }
}
if (report && report.dry_run && !Array.isArray(report.next_actions)) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_missing_next_actions' });
if (report && report.apply === false && Number(report.executed_step_count || 0) !== 0) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_executed_runtime_steps', executed: report.executed_step_count });
if (report && report.apply === false && Number(report.skipped_mutating_step_count || 0) < 1) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_did_not_skip_mutating_steps' });
if (report && report.ok !== true) violations.push({ kind: 'gateway_disposable_live_replay_report_not_ok', diagnostic: report.diagnostic });

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report?.trace_id || null,
  source_domain: 'validation',
  type: 'gateway_disposable_live_replay_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyPath,
  report_path: policy.report_path,
  replay_diagnostic: report?.diagnostic || null,
  apply: report?.apply ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/gateway_disposable_live_replay_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
