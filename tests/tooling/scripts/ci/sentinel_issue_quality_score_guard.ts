import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/evals/sentinel_issue_quality_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportRel = String(policy.report_path || "core/local/artifacts/sentinel_issue_quality_score_current.json");
const guardRel = String(policy.guard_report_path || "core/local/artifacts/sentinel_issue_quality_score_guard_current.json");
const historyRel = String(policy.history_path || "local/state/kernel_sentinel/issue_quality_score_history.jsonl");
const reportPath = path.join(root, reportRel);
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const candidates = Array.isArray(report.candidates) ? (report.candidates as Json[]) : [];
const violations: string[] = [];
const requiredFields = Array.isArray(policy.required_candidate_fields)
  ? policy.required_candidate_fields.map(String)
  : [];
const requiredDimensions = Array.isArray(policy.required_score_dimensions)
  ? policy.required_score_dimensions.map(String)
  : [];
const promotionThreshold = Number(policy.promotion_threshold || policy.minimum_promotion_ready_score || 80);
const reviewThreshold = Number(policy.review_threshold || policy.minimum_human_review_score || 60);

function exists(rel: string): boolean {
  try {
    return fs.statSync(path.join(root, rel)).isFile();
  } catch {
    return false;
  }
}

function historyRows(): Json[] {
  try {
    return fs
      .readFileSync(path.join(root, historyRel), "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line) as Json);
  } catch {
    return [];
  }
}

function candidateId(candidate: Json): string {
  return String(candidate.id || candidate.fingerprint || "unknown");
}

if (!report.trace_id) violations.push("missing_trace_id");
if (report.source_domain !== "validation") violations.push("wrong_source_domain");
if (report.type !== "sentinel_issue_quality_score") violations.push("wrong_report_type");
if (typeof report.candidate_count !== "number") violations.push("missing_candidate_count");
if (typeof report.average_score !== "number") violations.push("missing_average_score");
if (!report.quality_trend || typeof report.quality_trend !== "object") violations.push("missing_quality_trend");
if (!exists(historyRel)) violations.push("missing_history_file");
if (historyRows().length === 0) violations.push("empty_history_file");

for (const candidate of candidates) {
  const id = candidateId(candidate);
  for (const field of requiredFields) {
    if (!(field in candidate)) violations.push(`candidate_${id}_missing_${field}`);
  }
  const dimensions = (candidate.score_dimensions || {}) as Json;
  for (const dimension of requiredDimensions) {
    if (typeof dimensions[dimension] !== "number") {
      violations.push(`candidate_${id}_missing_dimension_${dimension}`);
    }
  }
  const evidenceRefs = Array.isArray(candidate.evidence_refs) ? candidate.evidence_refs.map(String) : [];
  const traceIds = Array.isArray(candidate.trace_ids) ? candidate.trace_ids.map(String).filter(Boolean) : [];
  const failures = Array.isArray(candidate.quality_failure_reasons)
    ? candidate.quality_failure_reasons.map(String)
    : [];
  const score = Number(candidate.score || 0);
  const state = String(candidate.promotion_state || "");
  if (state === "promotion_ready") {
    if (score < promotionThreshold) violations.push(`candidate_${id}_promotion_below_threshold`);
    if (failures.length > 0) violations.push(`candidate_${id}_promotion_has_quality_failures`);
    if (evidenceRefs.length === 0) violations.push(`candidate_${id}_promotion_missing_evidence`);
    if (traceIds.length === 0) violations.push(`candidate_${id}_promotion_missing_trace`);
    if (!candidate.owner_guess) violations.push(`candidate_${id}_promotion_missing_owner`);
    if (!candidate.root_cause_hypothesis) violations.push(`candidate_${id}_promotion_missing_root_cause`);
    if (!candidate.next_action) violations.push(`candidate_${id}_promotion_missing_next_action`);
    if (Number(candidate.recurrence_count || 0) < 2) violations.push(`candidate_${id}_promotion_not_recurrent`);
  }
  if (state === "human_review" && traceIds.length === 0) {
    violations.push(`candidate_${id}_review_missing_trace`);
  }
  if (state === "human_review" && score < reviewThreshold) {
    violations.push(`candidate_${id}_review_below_threshold`);
  }
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:sentinel-issue-quality-guard`,
  parent_span_id: report.trace_id || null,
  source_domain: "validation",
  type: "sentinel_issue_quality_score_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: reportRel,
  history_path: historyRel,
  candidate_count: report.candidate_count || 0,
  promotion_ready_count: report.promotion_ready_count || 0,
  human_review_count: report.human_review_count || 0,
  observation_only_count: report.observation_only_count || 0,
  average_score: report.average_score ?? null,
  violations,
};
const guardPath = path.join(root, guardRel);
fs.mkdirSync(path.dirname(guardPath), { recursive: true });
fs.writeFileSync(guardPath, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
