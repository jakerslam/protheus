import fs from "node:fs";
import path from "node:path";

type JsonRecord = Record<string, unknown>;

const repoRoot = process.cwd();
const policyPath = path.join(repoRoot, "observability/sentinel/sentinel_full_run_stage_split_policy.json");
const reportPath = path.join(
  repoRoot,
  "observability/reports",
  `sentinel_full_run_stage_split_plan_${new Date().toISOString().slice(0, 10)}.json`,
);

function readJson(filePath: string): JsonRecord | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as JsonRecord;
  } catch {
    return null;
  }
}

const policy = readJson(policyPath);
const report = readJson(reportPath);
const violations: string[] = [];

if (!policy) {
  violations.push("missing_stage_split_policy");
}
if (!report) {
  violations.push("missing_stage_split_report");
}

const requiredPhaseCount = Number(policy?.required_phase_count || 0);
const plannedPhaseCount = Number(report?.planned_phase_count || 0);
if (plannedPhaseCount < requiredPhaseCount) {
  violations.push("planned_phase_count_below_policy");
}

if (policy?.requires_partial_timing_artifact !== true) {
  violations.push("policy_must_require_partial_timing_artifact");
}
if (policy?.requires_resume_cursor !== true) {
  violations.push("policy_must_require_resume_cursor");
}
if (policy?.requires_raw_evidence_stream_not_final_report !== true) {
  violations.push("policy_must_keep_raw_evidence_out_of_final_report");
}
if (report?.plan_ready !== true) {
  violations.push("stage_split_plan_not_ready");
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:sentinel-stage-split-guard`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "sentinel_full_run_stage_split_plan_guard",
  generated_at: new Date().toISOString(),
  policy_path: "observability/sentinel/sentinel_full_run_stage_split_policy.json",
  report_path: path.relative(repoRoot, reportPath),
  violations,
};

console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) {
  process.exitCode = 1;
}
