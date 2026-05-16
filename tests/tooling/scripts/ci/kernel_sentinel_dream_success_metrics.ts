import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyRel = "observability/sentinel/sentinel_dream_success_metrics_policy.json";
const policy = JSON.parse(fs.readFileSync(path.join(root, policyRel), "utf8")) as Json;
const inputs = (policy.inputs || {}) as Record<string, string>;
const outputRel = String(policy.output_path || "core/local/artifacts/kernel_sentinel_dream_success_metrics_current.json");
const strict = process.argv.includes("--strict=1");

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function writeJson(rel: string, payload: unknown): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

const stageRunner = readJson(inputs.stage_runner_report);
const issueQuality = readJson(inputs.issue_quality_report);
const feedbackSummary = readJson(inputs.feedback_summary);
const freshGuard = readJson(inputs.fresh_evidence_guard);
const traceGuard = readJson(inputs.trace_completeness_guard);
const retention = readJson(inputs.retention_report);

const checks = [
  { id: "all_stages_complete", ok: stageRunner?.ok === true && Number(stageRunner?.remaining_phase_count || 0) === 0, weight: 25 },
  { id: "issue_quality_scored", ok: issueQuality?.ok === true && typeof issueQuality?.average_score === "number", weight: 20 },
  { id: "feedback_summary_ready", ok: feedbackSummary?.ok === true, weight: 20 },
  { id: "freshness_guard_ok", ok: freshGuard?.ok === true, weight: 15 },
  { id: "trace_guard_ok", ok: traceGuard?.ok === true, weight: 15 },
  { id: "retention_checked", ok: retention?.ok === true, weight: 5 },
];
const score = checks.reduce((sum, check) => sum + (check.ok ? check.weight : 0), 0);
const threshold = Number(policy.success_threshold || 85);
const violations = checks.filter((check) => !check.ok).map((check) => `failed_${check.id}`);

const result = {
  trace_id: `observability:${new Date().toISOString()}:kernel-sentinel-dream-success-metrics`,
  parent_span_id: String(stageRunner?.trace_id || issueQuality?.trace_id || ""),
  source_domain: "observability",
  type: "kernel_sentinel_dream_success_metrics",
  generated_at: new Date().toISOString(),
  ok: score >= threshold && violations.length === 0,
  policy_path: policyRel,
  score,
  threshold,
  checks,
  stage_remaining_phase_count: Number(stageRunner?.remaining_phase_count ?? -1),
  issue_candidate_count: Number(issueQuality?.candidate_count || 0),
  actionable_feedback_count: Number(feedbackSummary?.actionable_feedback_count || 0),
  feedback_status: feedbackSummary?.status || "unknown",
  retention_archive_candidate_count: Number(retention?.archive_candidate_count || 0),
  violations,
  source_refs: inputs,
};

writeJson(outputRel, result);
console.log(JSON.stringify(result, null, 2));
if (strict && result.ok !== true) process.exitCode = 1;
