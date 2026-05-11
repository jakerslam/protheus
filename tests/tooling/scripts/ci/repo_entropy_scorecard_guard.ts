import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;

const root = process.cwd();
const policyPath = path.join(root, 'validation/scorecards/repo_entropy_scorecard_policy.json');
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Json;
const reportRelPath = String(policy.report_path || 'core/local/artifacts/repo_entropy_scorecard_current.json');
const reportPath = path.join(root, reportRelPath);
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as Json;
const dimensions = Array.isArray(report.dimensions) ? report.dimensions as Json[] : [];
const violations: string[] = [];

for (const name of (policy.required_dimensions as string[]) || []) {
  if (!dimensions.some((row) => row.name === name)) violations.push(`missing_dimension_${name}`);
}
if (!report.trace_id) violations.push('missing_trace_id');
if (report.source_domain !== 'validation') violations.push('wrong_source_domain');
if (report.type !== 'repo_entropy_scorecard') violations.push('wrong_report_type');
if (!report.summary || typeof report.summary !== 'object') violations.push('missing_summary');
if (!Array.isArray(report.artifact_paths) || report.artifact_paths.length < 2) violations.push('missing_artifact_paths');

const requiredSummaryKeys = [
  'dirty_paths',
  'npm_scripts',
  'command_entries',
  'workflow_files',
  'required_ci_checks',
  'core_local_artifacts',
  'core_local_artifact_bytes',
  'effective_loc',
  'effective_loc_delta',
  'effective_loc_delta_pct',
  'guard_scripts',
  'gate_registry_entries',
  'duplicate_surface_roots',
];
for (const key of requiredSummaryKeys) {
  if (typeof report.summary?.[key] !== 'number') violations.push(`missing_numeric_summary_${key}`);
}

for (const row of dimensions) {
  if (!row.metric_key) violations.push(`dimension_missing_metric_key_${row.name || 'unknown'}`);
  if (typeof row.value !== 'number') violations.push(`dimension_missing_numeric_value_${row.name || 'unknown'}`);
  if (!['pass', 'white', 'yellow', 'red'].includes(String(row.severity))) violations.push(`dimension_invalid_severity_${row.name || 'unknown'}`);
  if ((row.severity === 'red' || row.severity === 'yellow') && (!Array.isArray(row.next_actions) || row.next_actions.length === 0)) {
    violations.push(`dimension_missing_next_actions_${row.name || 'unknown'}`);
  }
}

const redDimensions = Array.isArray(report.red_dimensions) ? report.red_dimensions : [];
if (redDimensions.length > 0 && !dimensions.some((row) => row.severity === 'red')) violations.push('red_dimensions_mismatch');

const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:repo-entropy-scorecard-guard`;
const result = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: 'validation',
  type: 'repo_entropy_scorecard_guard',
  generated_at: generatedAt,
  ok: violations.length === 0,
  policy_path: path.relative(root, policyPath).replace(/\\/g, '/'),
  report_path: reportRelPath,
  scorecard_severity: report.severity || null,
  entropy_score: report.entropy_score || 0,
  red_dimensions: report.red_dimensions || [],
  yellow_dimensions: report.yellow_dimensions || [],
  violation_count: violations.length,
  violations,
};

const out = path.join(root, 'core/local/artifacts/repo_entropy_scorecard_guard_current.json');
fs.mkdirSync(path.dirname(out), { recursive: true });
fs.writeFileSync(out, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
