#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/reports (Rust crate support evidence report)

const fs = require('fs');
const path = require('path');
const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/rust_crate_support_evidence_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const manifest = JSON.parse(fs.readFileSync(path.join(root, policy.manifest_path), 'utf8'));
const crates = Array.isArray(manifest.crates) ? manifest.crates : [];
const cargoReportPath = path.join(root, policy.cargo_check_report_path || '');
const cargoReport = policy.cargo_check_report_path && fs.existsSync(cargoReportPath)
  ? JSON.parse(fs.readFileSync(cargoReportPath, 'utf8'))
  : null;
const cargoEvidenceByPath = new Map((cargoReport?.rows || []).map((row) => [row.path, row]));
const exemptionByPath = new Map((policy.production_candidate_validation_exemptions || []).map((row) => [row.path, row]));

function inferEvidence(row) {
  const full = path.join(root, row.path);
  const exists = fs.existsSync(full);
  const source = exists ? fs.readFileSync(full, 'utf8') : '';
  const hasPackage = /\[package\]/.test(source);
  const hasName = /^name\s*=\s*"[^"]+"/m.test(source);
  const hasVersion = /^version\s*=\s*"[^"]+"/m.test(source);
  const status = !exists ? 'blocked_missing_manifest' : (hasPackage && hasName && hasVersion ? 'manifest_parse_ready' : 'manifest_incomplete');
  const cargoEvidence = cargoEvidenceByPath.get(row.path) || null;
  const exemption = exemptionByPath.get(row.path) || null;
  const validationEvidenceStatus = row.support_status !== 'production_candidate'
    ? 'not_required'
    : exemption
      ? 'explicit_exemption'
      : cargoEvidence?.evidence_status || 'missing_validation_evidence';
  const validationEvidenceKind = row.support_status !== 'production_candidate'
    ? 'not_required'
    : exemption
      ? 'explicit_exemption'
      : cargoEvidence
        ? 'cargo_check'
        : 'missing';
  const recommendedStatus = !exists
    ? 'blocked'
    : row.support_status === 'production_candidate' && status !== 'manifest_parse_ready'
      ? 'experimental'
      : row.support_status === 'production_candidate' && !['cargo_check_passed', 'cargo_test_passed', 'explicit_exemption'].includes(validationEvidenceStatus)
        ? 'experimental'
      : row.support_status;
  return {
    path: row.path,
    domain: row.domain,
    owner: row.owner,
    support_status: row.support_status,
    evidence_status: status,
    validation_evidence_status: validationEvidenceStatus,
    validation_evidence_kind: validationEvidenceKind,
    validation_evidence_command: cargoEvidence?.command || null,
    validation_evidence_duration_ms: typeof cargoEvidence?.duration_ms === 'number' ? cargoEvidence.duration_ms : null,
    validation_evidence_exit_code: typeof cargoEvidence?.exit_code === 'number' ? cargoEvidence.exit_code : null,
    validation_exemption: exemption || null,
    recommended_status: recommendedStatus,
    cargo_manifest_exists: exists,
    has_package_section: hasPackage,
    has_name: hasName,
    has_version: hasVersion,
  };
}
const rows = crates.map(inferEvidence);
const summary = rows.reduce((acc, row) => {
  acc.by_support_status[row.support_status] = (acc.by_support_status[row.support_status] || 0) + 1;
  acc.by_evidence_status[row.evidence_status] = (acc.by_evidence_status[row.evidence_status] || 0) + 1;
  acc.by_validation_evidence_status[row.validation_evidence_status] = (acc.by_validation_evidence_status[row.validation_evidence_status] || 0) + 1;
  acc.recommendation_count += row.recommended_status !== row.support_status ? 1 : 0;
  return acc;
}, { by_support_status: {}, by_evidence_status: {}, by_validation_evidence_status: {}, recommendation_count: 0 });
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: cargoReport?.trace_id || null,
  source_domain: 'validation',
  type: 'rust_crate_support_evidence_report',
  generated_at: new Date().toISOString(),
  policy_path: policyPath,
  cargo_check_report_path: policy.cargo_check_report_path || null,
  cargo_check_report_loaded: Boolean(cargoReport),
  crate_count: rows.length,
  production_candidate_count: rows.filter((row) => row.support_status === 'production_candidate').length,
  production_candidate_validated_count: rows.filter((row) => row.support_status === 'production_candidate' && ['cargo_check_passed', 'cargo_test_passed', 'explicit_exemption'].includes(row.validation_evidence_status)).length,
  summary,
  crates: rows,
};
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({
  ok: true,
  type: 'rust_crate_support_evidence_report',
  report_path: policy.report_path,
  crate_count: rows.length,
  production_candidate_count: payload.production_candidate_count,
  production_candidate_validated_count: payload.production_candidate_validated_count,
  recommendation_count: summary.recommendation_count,
}, null, 2));
