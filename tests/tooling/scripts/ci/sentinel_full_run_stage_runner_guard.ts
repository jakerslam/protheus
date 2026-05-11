import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "observability/sentinel/sentinel_full_run_stage_split_policy.json");
const runnerPath = path.join(root, "observability/sentinel/sentinel_full_run_stage_runner.ts");
const reportPath = path.join(root, "observability/reports/sentinel_full_run_stage_runner_current.json");
const statePath = path.join(root, "local/state/observability/sentinel/full_run_stage_state_current.json");

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as Json;
  } catch {
    return null;
  }
}

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, "utf8");
  } catch {
    return "";
  }
}

const policy = readJson(policyPath);
const report = readJson(reportPath);
const state = readJson(statePath);
const runner = readText(runnerPath);
const violations: string[] = [];

if (!policy) violations.push("missing_stage_split_policy");
if (!report) violations.push("missing_stage_runner_report");
if (!state) violations.push("missing_stage_runner_state");
if (!runner.includes("resume_cursor")) violations.push("runner_missing_resume_cursor");
if (!runner.includes("raw evidence remains in source streams")) violations.push("runner_must_keep_raw_evidence_in_source_streams");
if (!runner.includes("sentinel_staged_compact_report")) violations.push("runner_missing_compact_report_output");
if (report?.ok !== true) violations.push("stage_runner_report_not_ok");
if (String(report?.source_domain || "") !== "observability") violations.push("stage_runner_wrong_source_domain");
if (!Array.isArray(report?.phase_results)) violations.push("stage_runner_missing_phase_results");
if (!Array.isArray(state?.completed_phases)) violations.push("stage_runner_state_missing_completed_phases");
if (!Array.isArray(state?.remaining_phases)) violations.push("stage_runner_state_missing_remaining_phases");

const executed = Number(report?.executed_phase_count || 0);
const sampleOnlyComplete =
  report?.sample_only === true &&
  Number(report?.remaining_phase_count || 0) === 0 &&
  Number(report?.completed_phase_count || 0) >= Number(policy?.required_phase_count || 0) &&
  Array.isArray(report?.timing_sample_refs) &&
  Array.isArray(report?.all_phase_results) &&
  report.all_phase_results.length >= Number(policy?.required_phase_count || 0);
if (executed < 1 && !sampleOnlyComplete) violations.push("stage_runner_executed_no_phases");

const result = {
  trace_id: `validation:${new Date().toISOString()}:sentinel-stage-runner-guard`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "sentinel_full_run_stage_runner_guard",
  generated_at: new Date().toISOString(),
  report_path: path.relative(root, reportPath),
  state_path: path.relative(root, statePath),
  violations,
};

console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
