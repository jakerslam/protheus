import fs from 'node:fs';
import path from 'node:path';

type Json = Record<string, any>;

const root = process.cwd();
const policyRelPath = 'validation/proof_packs/real_work_workflow_proof_policy.json';
const policyPath = path.join(root, policyRelPath);
const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8')) as Json;
const reportRelPath = String(policy.report_path || 'core/local/artifacts/real_work_workflow_proof_current.json');
const reportPath = path.join(root, reportRelPath);
const report = JSON.parse(fs.readFileSync(reportPath, 'utf8')) as Json;
const lanes = Array.isArray(report.lanes) ? report.lanes as Json[] : [];
const violations: string[] = [];

if (!report.trace_id) violations.push('missing_trace_id');
if (report.source_domain !== 'validation') violations.push('wrong_source_domain');
if (report.type !== 'real_work_workflow_proof') violations.push('wrong_report_type');
if (report.ok !== true) violations.push('real_work_proof_below_minimums');
if (Number(report.ready_lane_count || 0) < Number(policy.minimum_ready_lanes || 1)) violations.push('ready_lane_count_below_minimum');
if (Number(report.user_visible_ready_lane_count || 0) < Number(policy.minimum_user_visible_lanes || 1)) violations.push('user_visible_ready_lane_count_below_minimum');
if (Number(report.live_ready_lane_count || 0) < Number(policy.minimum_live_lanes || 0)) violations.push('live_ready_lane_count_below_minimum');
if (Number(report.distinct_ready_work_class_count || 0) < Number(policy.minimum_distinct_work_classes || 1)) {
  violations.push('distinct_ready_work_class_count_below_minimum');
}
if (Number(report.distinct_ready_capability_domain_count || 0) < Number(policy.minimum_distinct_capability_domains || 1)) {
  violations.push('distinct_ready_capability_domain_count_below_minimum');
}
if (policy?.policy?.proof_summary_must_name_capability_outcomes === true && !Array.isArray(report.capability_outcomes)) {
  violations.push('missing_capability_outcomes');
}
for (const row of lanes) {
  for (const field of (policy.required_lane_fields as string[]) || []) {
    if (!(field in row)) violations.push(`lane_${row.id || 'unknown'}_missing_${field}`);
  }
  if (row.ready === true) {
    if (row.evidence_ok !== true) violations.push(`lane_${row.id}_ready_without_passing_evidence`);
    if (row.guard_artifact_ok !== true) violations.push(`lane_${row.id}_ready_without_passing_guard_artifact`);
    if (!row.source_guard_path) violations.push(`lane_${row.id}_ready_without_source_guard`);
    if (row.fresh !== true) violations.push(`lane_${row.id}_ready_without_fresh_evidence`);
    if (policy?.policy?.ready_lanes_must_explain_user_value === true && !row.user_value_statement) {
      violations.push(`lane_${row.id}_ready_without_user_value_statement`);
    }
    if (policy?.policy?.ready_lanes_must_have_end_to_end_chain === true) {
      const chain = row.end_to_end_chain && typeof row.end_to_end_chain === 'object' ? row.end_to_end_chain as Json : {};
      for (const field of (policy.required_ready_chain_fields as string[]) || []) {
        if (!chain[field]) violations.push(`lane_${row.id}_ready_chain_missing_${field}`);
      }
    }
  } else if (!row.next_action) {
    violations.push(`lane_${row.id || 'unknown'}_missing_next_action`);
  }
}
const readyDomains = new Set(lanes.filter((row) => row.ready === true).map((row) => row.capability_domain));
for (const domain of (policy.required_ready_domains as string[]) || ['gateway', 'tooling', 'installer', 'validation']) {
  if (!readyDomains.has(domain)) violations.push(`missing_ready_domain_${domain}`);
}
const readyWorkClasses = new Set(lanes.filter((row) => row.ready === true).map((row) => row.work_class));
for (const workClass of (policy.required_ready_work_classes as string[]) || []) {
  if (!readyWorkClasses.has(workClass)) violations.push(`missing_ready_work_class_${workClass}`);
}
const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:real-work-workflow-proof-guard`;
const result = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: report.trace_id || null,
  source_domain: 'validation',
  type: 'real_work_workflow_proof_guard',
  generated_at: generatedAt,
  ok: violations.length === 0,
  policy_path: policyRelPath,
  report_path: reportRelPath,
  ready_lane_count: report.ready_lane_count || 0,
  user_visible_ready_lane_count: report.user_visible_ready_lane_count || 0,
  live_ready_lane_count: report.live_ready_lane_count || 0,
  distinct_ready_work_class_count: report.distinct_ready_work_class_count || 0,
  distinct_ready_capability_domain_count: report.distinct_ready_capability_domain_count || 0,
  ready_work_classes: report.ready_work_classes || [],
  ready_capability_domains: report.ready_capability_domains || [],
  total_lane_count: report.total_lane_count || 0,
  violation_count: violations.length,
  violations,
};
const outRel = String(policy.guard_result_path || 'core/local/artifacts/real_work_workflow_proof_guard_current.json');
const outAbs = path.join(root, outRel);
fs.mkdirSync(path.dirname(outAbs), { recursive: true });
fs.writeFileSync(outAbs, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
