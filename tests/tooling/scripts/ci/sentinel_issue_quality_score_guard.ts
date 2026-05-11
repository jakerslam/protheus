import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/evals/sentinel_issue_quality_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/sentinel_issue_quality_score_current.json"));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const candidates = Array.isArray(report.candidates) ? report.candidates as Json[] : [];
const violations: string[] = [];

if (!report.trace_id) violations.push("missing_trace_id");
if (report.source_domain !== "validation") violations.push("wrong_source_domain");
if (typeof report.candidate_count !== "number") violations.push("missing_candidate_count");
for (const candidate of candidates) {
  for (const field of (policy.required_candidate_fields as string[]) || []) {
    if (!(field in candidate)) violations.push(`candidate_${candidate.id || "unknown"}_missing_${field}`);
  }
  if (candidate.promotion_state === "promotion_ready" && Number(candidate.score || 0) < Number(policy.promotion_threshold || 80)) {
    violations.push(`candidate_${candidate.id || "unknown"}_promotion_below_threshold`);
  }
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:sentinel-issue-quality-guard`,
  source_domain: "validation",
  type: "sentinel_issue_quality_score_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  candidate_count: report.candidate_count || 0,
  promotion_ready_count: report.promotion_ready_count || 0,
  human_review_count: report.human_review_count || 0,
  observation_only_count: report.observation_only_count || 0,
  violations,
};
fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/sentinel_issue_quality_score_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
