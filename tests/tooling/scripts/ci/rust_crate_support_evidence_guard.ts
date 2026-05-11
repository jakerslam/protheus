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
const acceptedValidationStatuses = new Set(policy.accepted_validation_evidence_statuses || []);
if (!report) violations.push({ kind: 'rust_crate_support_evidence_report_missing', path: policy.report_path });
if (report && policy.cargo_check_report_path && !report.cargo_check_report_loaded) {
  violations.push({ kind: 'rust_crate_cargo_check_evidence_report_missing', path: policy.cargo_check_report_path });
}
for (const row of (report && report.crates) || []) {
  if (row.support_status === 'production_candidate' && row.evidence_status !== 'manifest_parse_ready') {
    violations.push({ kind: 'production_candidate_without_manifest_parse_evidence', path: row.path, evidence_status: row.evidence_status, recommended_status: row.recommended_status });
  }
  if (row.support_status === 'production_candidate' && !acceptedValidationStatuses.has(row.validation_evidence_status)) {
    violations.push({
      kind: 'production_candidate_without_validation_evidence',
      path: row.path,
      validation_evidence_status: row.validation_evidence_status,
      validation_evidence_kind: row.validation_evidence_kind,
      recommended_status: row.recommended_status,
    });
  }
  if (row.support_status === 'production_candidate' && row.validation_evidence_status === 'explicit_exemption') {
    const exemption = row.validation_exemption || {};
    for (const field of ['reason', 'owner', 'expires_on']) {
      if (!exemption[field]) {
        violations.push({ kind: 'production_candidate_validation_exemption_incomplete', path: row.path, missing: field });
      }
    }
  }
  if (row.evidence_status === 'blocked_missing_manifest') {
    violations.push({ kind: 'rust_crate_manifest_missing', path: row.path, recommended_status: row.recommended_status });
  }
}
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report?.trace_id || null,
  source_domain: 'validation',
  ok: violations.length === 0,
  type: 'rust_crate_support_evidence_guard',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  report_path: policy.report_path,
  cargo_check_report_path: policy.cargo_check_report_path || null,
  production_candidate_count: report?.production_candidate_count ?? null,
  production_candidate_validated_count: report?.production_candidate_validated_count ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/rust_crate_support_evidence_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
