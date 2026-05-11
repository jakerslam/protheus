#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/release_gates (CI branch protection projection guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/release_gates/policies/ci_branch_protection_projection_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];

if (!report) violations.push({ kind: 'ci_branch_projection_report_missing', path: policy.report_path });
if (report && report.source_domain !== 'validation') violations.push({ kind: 'ci_branch_projection_wrong_source_domain', actual: report.source_domain });
if (report && report.projected_required_count > policy.target_required_max) violations.push({ kind: 'ci_branch_projection_required_count_over_budget', actual: report.projected_required_count, max: policy.target_required_max });
if (report && !Array.isArray(report.required_contexts)) violations.push({ kind: 'ci_branch_projection_missing_required_contexts' });
if (report && !Array.isArray(report.advisory_contexts)) violations.push({ kind: 'ci_branch_projection_missing_advisory_contexts' });
if (report && report.requires_human_review_before_apply !== true) violations.push({ kind: 'ci_branch_projection_missing_human_review_gate' });
if (report && (!Array.isArray(report.next_actions) || report.next_actions.length < 1)) violations.push({ kind: 'ci_branch_projection_missing_next_actions' });

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report?.trace_id || null,
  source_domain: 'validation',
  type: 'ci_branch_protection_projection_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyPath,
  report_path: policy.report_path,
  projected_required_count: report?.projected_required_count ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/ci_branch_protection_projection_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
