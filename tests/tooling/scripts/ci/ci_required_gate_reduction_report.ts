#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/reports (CI required gate reduction planner)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/release_gates/policies/ci_required_gate_reduction_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const manifest = JSON.parse(fs.readFileSync(path.join(root, policy.manifest_path), 'utf8'));
const workflows = Array.isArray(manifest.workflows) ? manifest.workflows : [];
function hasToken(row, tokens) {
  const text = `${row.file || ''}\n${row.name || ''}\n${row.tier || ''}`.toLowerCase();
  return (tokens || []).some((token) => text.includes(String(token).toLowerCase()));
}
const required = workflows.filter((row) => row.required_for_release);
const keepRequired = [];
const demotionCandidates = [];
for (const row of required) {
  if (hasToken(row, policy.always_required_name_tokens)) keepRequired.push({ ...row, reason: 'always_required_token' });
  else if (hasToken(row, policy.prefer_advisory_or_nightly_tokens)) demotionCandidates.push({ ...row, recommended_tier: row.tier === 'security_gate' ? 'observability_guard' : 'nightly_maintenance', reason: 'expensive_or_non_daily_signal' });
  else demotionCandidates.push({ ...row, recommended_tier: 'advisory_guard', reason: 'not_in_minimal_required_set' });
}
const target = Number(policy.target_required_max) || 18;
const requiredAfterPlan = Math.max(keepRequired.length, target);
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'ci_required_gate_reduction_plan',
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  current_required_count: required.length,
  target_required_max: target,
  keep_required_count: keepRequired.length,
  recommended_demotion_count: demotionCandidates.length,
  required_after_plan_floor: requiredAfterPlan,
  status: required.length <= target ? 'within_budget' : 'reduction_needed',
  keep_required: keepRequired,
  demotion_candidates: demotionCandidates,
  next_action: 'Review demotion candidates, then update ci_workflow_tier_manifest.json and branch protection required checks together.'
};
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: payload.type, status: payload.status, current_required_count: payload.current_required_count, target_required_max: target, recommended_demotion_count: payload.recommended_demotion_count, report_path: policy.report_path }, null, 2));
