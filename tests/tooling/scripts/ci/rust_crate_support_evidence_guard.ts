#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (Rust crate support evidence guard)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/rust_crate_support_evidence_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
if (!report) violations.push({ kind: 'rust_crate_support_evidence_report_missing', path: policy.report_path });
for (const row of (report && report.crates) || []) {
  if (row.support_status === 'production_candidate' && row.evidence_status !== 'manifest_parse_ready') {
    violations.push({ kind: 'production_candidate_without_manifest_parse_evidence', path: row.path, evidence_status: row.evidence_status, recommended_status: row.recommended_status });
  }
  if (row.evidence_status === 'blocked_missing_manifest') {
    violations.push({ kind: 'rust_crate_manifest_missing', path: row.path, recommended_status: row.recommended_status });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', ok: violations.length === 0, type: 'rust_crate_support_evidence_guard', generated_at: new Date().toISOString(), policy_path: policyPath, report_path: policy.report_path, violations };
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/rust_crate_support_evidence_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
