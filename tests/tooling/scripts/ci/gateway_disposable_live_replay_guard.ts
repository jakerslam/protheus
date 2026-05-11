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
const liveProofRequired = policy.live_proof_required_for_completion === true;
const liveProofMaxAgeMs = Number(policy.live_proof_max_age_ms || 86400000);

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
  if (!JSON.stringify(step.command || []).includes('${INFRING_OPS}')) {
    violations.push({ kind: 'gateway_disposable_live_replay_not_source_ops_binary_routed', id: step.id });
  }
  if (JSON.stringify(step.command || []).includes('"npm"')) {
    violations.push({ kind: 'gateway_disposable_live_replay_uses_npm_launcher', id: step.id });
  }
  if (step.kind === 'status' && !JSON.stringify(step.command || []).includes('--auto-heal=0')) {
    violations.push({ kind: 'gateway_disposable_live_replay_status_not_read_only', id: step.id });
  }
  if ((step.kind === 'start' || step.kind === 'restart') && !JSON.stringify(step.command || []).includes('--dashboard-open=0')) {
    violations.push({ kind: 'gateway_disposable_live_replay_may_auto_open_dashboard', id: step.id });
  }
  if ((step.kind === 'start' || step.kind === 'restart') && !JSON.stringify(step.command || []).includes('--gateway-persist=0')) {
    violations.push({ kind: 'gateway_disposable_live_replay_may_leave_persistent_supervisor', id: step.id });
  }
}
if (report && report.dry_run && !Array.isArray(report.next_actions)) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_missing_next_actions' });
if (report && report.apply === false && Number(report.executed_step_count || 0) !== 0) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_executed_runtime_steps', executed: report.executed_step_count });
if (report && report.apply === false && Number(report.skipped_mutating_step_count || 0) < 1) violations.push({ kind: 'gateway_disposable_live_replay_dry_run_did_not_skip_mutating_steps' });
if (report && liveProofRequired) {
  const ageMs = Date.now() - Date.parse(String(report.generated_at || ''));
  if (report.apply !== true) violations.push({ kind: 'gateway_disposable_live_replay_live_proof_missing', diagnostic: report.diagnostic });
  if (report.live_proof_complete !== true) violations.push({ kind: 'gateway_disposable_live_replay_live_proof_incomplete', completed: report.live_completed_step_ids || [] });
  if (!Number.isFinite(ageMs) || ageMs > liveProofMaxAgeMs) violations.push({ kind: 'gateway_disposable_live_replay_live_proof_stale', age_ms: Number.isFinite(ageMs) ? ageMs : null, max_age_ms: liveProofMaxAgeMs });
  if (!report.ops_binary) violations.push({ kind: 'gateway_disposable_live_replay_missing_ops_binary_resolution' });
}
if (report && Array.isArray(report.setup_results) && report.setup_results.some((row) => row.ok !== true)) {
  violations.push({ kind: 'gateway_disposable_live_replay_setup_failed', setup_results: report.setup_results });
}
if (report && Array.isArray(report.results)) {
  for (const step of report.results) {
    if (step.skipped || step.diagnostic_only) continue;
    if (Number(step.forbidden_token_match_count || 0) > 0) {
      violations.push({ kind: 'gateway_disposable_live_replay_success_step_matched_forbidden_tokens', id: step.id, tokens: step.forbidden_tokens });
    }
    if (Number(step.success_token_match_count || 0) === 0) {
      violations.push({ kind: 'gateway_disposable_live_replay_success_step_missing_success_tokens', id: step.id });
    }
    if (step.exit_code_ok !== true) {
      violations.push({ kind: 'gateway_disposable_live_replay_success_step_exit_not_accepted', id: step.id, exit_code: step.exit_code });
    }
    if (step.receipt_ok_satisfied !== true) {
      violations.push({ kind: 'gateway_disposable_live_replay_success_step_receipt_not_ok', id: step.id });
    }
    if (step.dashboard_running_satisfied !== true) {
      violations.push({ kind: 'gateway_disposable_live_replay_success_step_dashboard_not_running', id: step.id });
    }
  }
}
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
  live_proof_required: liveProofRequired,
  live_proof_complete: report?.live_proof_complete ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/gateway_disposable_live_replay_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
