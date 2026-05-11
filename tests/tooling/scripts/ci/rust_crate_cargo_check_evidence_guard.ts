#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0
// Layer ownership: validation/conformance (Rust crate cargo-check evidence guard)

const fs = require('fs');
const path = require('path');

const root = process.cwd();
const policyPath = path.join(root, 'validation/conformance/contracts/rust_crate_cargo_check_evidence_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));
const reportPath = path.join(root, policy.report_path);
const report = fs.existsSync(reportPath) ? JSON.parse(fs.readFileSync(reportPath, 'utf8')) : null;
const violations = [];
const accepted = new Set(policy.accepted_evidence_statuses || []);

if (!report) violations.push({ kind: 'rust_crate_cargo_check_report_missing', path: policy.report_path });
if (report && report.source_domain !== 'validation') violations.push({ kind: 'rust_crate_cargo_check_wrong_source_domain', actual: report.source_domain });
if (report && report.execute !== true) violations.push({ kind: 'rust_crate_cargo_check_not_executed' });
for (const row of (report?.rows || [])) {
  if (!accepted.has(row.evidence_status)) violations.push({ kind: 'rust_crate_cargo_check_unaccepted_status', path: row.path, evidence_status: row.evidence_status });
  if (!Array.isArray(row.command) || row.command.length < 4) violations.push({ kind: 'rust_crate_cargo_check_missing_command', path: row.path });
  if (typeof row.duration_ms !== 'number') violations.push({ kind: 'rust_crate_cargo_check_missing_duration', path: row.path });
}

const traceId = `validation:${new Date().toISOString()}:${process.pid}`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report?.trace_id || null,
  source_domain: 'validation',
  type: 'rust_crate_cargo_check_evidence_guard',
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  policy_path: policyPath,
  report_path: policy.report_path,
  observed_count: report?.observed_count ?? null,
  pass_count: report?.pass_count ?? null,
  fail_count: report?.fail_count ?? null,
  violations,
};
fs.mkdirSync(path.join(root, 'core/local/artifacts'), { recursive: true });
fs.writeFileSync(path.join(root, 'core/local/artifacts/rust_crate_cargo_check_evidence_guard_current.json'), `${JSON.stringify(payload, null, 2)}\n`);
console.log(JSON.stringify(payload, null, 2));
if (!payload.ok) process.exit(1);
