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
function inferEvidence(row) {
  const full = path.join(root, row.path);
  const exists = fs.existsSync(full);
  const source = exists ? fs.readFileSync(full, 'utf8') : '';
  const hasPackage = /\[package\]/.test(source);
  const hasName = /^name\s*=\s*"[^"]+"/m.test(source);
  const hasVersion = /^version\s*=\s*"[^"]+"/m.test(source);
  const status = !exists ? 'blocked_missing_manifest' : (hasPackage && hasName && hasVersion ? 'manifest_parse_ready' : 'manifest_incomplete');
  const recommendedStatus = !exists
    ? 'blocked'
    : row.support_status === 'production_candidate' && status !== 'manifest_parse_ready'
      ? 'experimental'
      : row.support_status;
  return { path: row.path, domain: row.domain, owner: row.owner, support_status: row.support_status, evidence_status: status, recommended_status: recommendedStatus, cargo_manifest_exists: exists, has_package_section: hasPackage, has_name: hasName, has_version: hasVersion };
}
const rows = crates.map(inferEvidence);
const summary = rows.reduce((acc, row) => {
  acc.by_support_status[row.support_status] = (acc.by_support_status[row.support_status] || 0) + 1;
  acc.by_evidence_status[row.evidence_status] = (acc.by_evidence_status[row.evidence_status] || 0) + 1;
  acc.recommendation_count += row.recommended_status !== row.support_status ? 1 : 0;
  return acc;
}, { by_support_status: {}, by_evidence_status: {}, recommendation_count: 0 });
const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = { trace_id: traceId, span_id: `span:${traceId}`, parent_span_id: null, source_domain: 'validation', type: 'rust_crate_support_evidence_report', generated_at: new Date().toISOString(), policy_path: policyPath, crate_count: rows.length, summary, crates: rows };
fs.mkdirSync(path.dirname(path.join(root, policy.report_path)), { recursive: true });
fs.writeFileSync(path.join(root, policy.report_path), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: 'rust_crate_support_evidence_report', report_path: policy.report_path, crate_count: rows.length, recommendation_count: summary.recommendation_count }, null, 2));
