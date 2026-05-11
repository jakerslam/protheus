import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/proof_packs/real_work_workflow_proof_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/real_work_workflow_proof_current.json"));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const lanes = Array.isArray(report.lanes) ? report.lanes as Json[] : [];
const violations: string[] = [];

if (!report.trace_id) violations.push("missing_trace_id");
if (report.source_domain !== "validation") violations.push("wrong_source_domain");
if (report.ok !== true) violations.push("real_work_proof_below_minimum_ready_lanes");
for (const row of lanes) {
  for (const field of (policy.required_lane_fields as string[]) || []) {
    if (!(field in row)) violations.push(`lane_${row.id || "unknown"}_missing_${field}`);
  }
}
const incomplete = lanes.filter((row) => row.ready !== true);
for (const row of incomplete) {
  if (!row.next_action) violations.push(`lane_${row.id || "unknown"}_missing_next_action`);
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:real-work-workflow-proof-guard`,
  source_domain: "validation",
  type: "real_work_workflow_proof_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  ready_lane_count: report.ready_lane_count || 0,
  total_lane_count: report.total_lane_count || 0,
  violations,
};
fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/real_work_workflow_proof_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
