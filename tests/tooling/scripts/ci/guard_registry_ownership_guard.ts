import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;

const root = process.cwd();
const policyRelPath = 'validation/conformance/contracts/guard_registry_ownership_policy.json';
const policyPath = path.join(root, policyRelPath);
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Json;
const reportRelPath = String(policy.report_path || 'core/local/artifacts/guard_registry_ownership_current.json');
const reportPath = path.join(root, reportRelPath);
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as Json;
const violations: string[] = [];

if (!report.trace_id) violations.push('missing_trace_id');
if (report.source_domain !== 'validation') violations.push('wrong_source_domain');
if (report.type !== 'guard_registry_ownership_report') violations.push('wrong_report_type');
if (!Array.isArray(report.rows)) violations.push('missing_guard_rows');
if (!Array.isArray(report.findings)) violations.push('missing_findings');
for (const key of [
  'guard_count',
  'registered_guard_count',
  'unregistered_guard_count',
  'stale_guard_candidate_count',
  'missing_ownership_count',
  'duplicate_family_count',
  'guard_artifact_count',
  'orphan_artifact_count',
]) {
  if (typeof report[key] !== 'number') violations.push(`missing_numeric_${key}`);
}
if (Number(report.guard_count || 0) < Number(report.registered_guard_count || 0)) violations.push('registered_count_exceeds_guard_count');
if (Number(report.missing_ownership_count || 0) > 0 && !report.findings.some((row: Json) => row.kind === 'guard_missing_ownership_marker')) {
  violations.push('missing_ownership_findings_absent');
}
if (Number(report.unregistered_guard_count || 0) > 0 && !report.findings.some((row: Json) => row.kind === 'unregistered_guard_script')) {
  violations.push('unregistered_guard_findings_absent');
}
if (Number(report.orphan_artifact_count || 0) > 0 && !report.findings.some((row: Json) => row.kind === 'orphan_guard_artifact')) {
  violations.push('orphan_artifact_findings_absent');
}
if (Number(report.duplicate_family_count || 0) > 0 && !Array.isArray(report.duplicate_families)) violations.push('duplicate_families_missing');
if (!['pass', 'white', 'yellow', 'red'].includes(String(report.severity || ''))) violations.push('invalid_severity');

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:guard-registry-ownership-guard`;
const result = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'guard_registry_ownership_guard',
  generated_at: generatedAt,
  ok: violations.length === 0,
  policy_path: policyRelPath,
  report_path: reportRelPath,
  scorecard_severity: report.severity || null,
  guard_count: report.guard_count || 0,
  registered_guard_count: report.registered_guard_count || 0,
  unregistered_guard_count: report.unregistered_guard_count || 0,
  stale_guard_candidate_count: report.stale_guard_candidate_count || 0,
  missing_ownership_count: report.missing_ownership_count || 0,
  duplicate_family_count: report.duplicate_family_count || 0,
  orphan_artifact_count: report.orphan_artifact_count || 0,
  violation_count: violations.length,
  violations,
};
const out = path.join(root, String(policy.guard_result_path || 'core/local/artifacts/guard_registry_ownership_guard_current.json'));
fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
