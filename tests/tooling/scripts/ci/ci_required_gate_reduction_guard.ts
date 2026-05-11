#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/release_gates (CI required gate reduction guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/release_gates/policies/ci_required_gate_reduction_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
if (!report) violations.push({ kind: 'ci_required_gate_reduction_report_missing', path: policy.report_path });
if (report && !['within_budget', 'reduction_needed'].includes(report.status)) violations.push({ kind: 'ci_required_gate_reduction_status_invalid', status: report.status });
if (report && report.current_required_count > policy.target_required_max && report.recommended_demotion_count < 1) violations.push({ kind: 'ci_required_gate_reduction_has_no_candidates' });
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'ci_required_gate_reduction_guard', generated_at: new Date().toISOString(), policy_path: policyPath, report_path: policy.report_path, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/ci_required_gate_reduction_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
