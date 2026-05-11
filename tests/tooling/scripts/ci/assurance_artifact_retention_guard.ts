import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
function arg(name: string, fallback: string): string {
  const prefix = `--${name}=`;
  const direct = process.argv.find((item) => item.startsWith(prefix));
  if (direct) return direct.slice(prefix.length);
  const idx = process.argv.indexOf(`--${name}`);
  return idx >= 0 && process.argv[idx + 1] ? process.argv[idx + 1] : fallback;
}

const reportRel = arg("report", `validation/reports/assurance_artifact_retention_report_${new Date().toISOString().slice(0, 10)}.json`);
const policyRel = arg("policy", "validation/conformance/contracts/assurance_artifact_retention_policy.json");
const reportPath = path.join(root, reportRel);
const policyPath = path.join(root, policyRel);

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as Json;
  } catch {
    return null;
  }
}

const policy = readJson(policyPath);
const report = readJson(reportPath);
const violations: string[] = [];
const advisories: string[] = [];
if (!report) violations.push("missing_assurance_artifact_retention_report");
if (report?.ok !== true) violations.push("retention_report_not_ok");
if (String(report?.source_domain || "") !== "validation") violations.push("retention_report_wrong_source_domain");
if (report && typeof report.trace_id !== "string") violations.push("retention_report_missing_trace_id");
if (report && typeof report.span_id !== "string") violations.push("retention_report_missing_span_id");
const requiredReportFields = Array.isArray(policy?.required_report_fields) ? policy.required_report_fields.map(String) : [];
for (const field of requiredReportFields) {
  if (report && !Object.prototype.hasOwnProperty.call(report, field)) violations.push(`retention_report_missing_required_field:${field}`);
}
const roots = Array.isArray(report?.roots) ? report.roots as Json[] : [];
if (roots.length < 3) violations.push("retention_report_missing_local_validation_or_observability_root");
if (!roots.some((row) => row.path === "core/local/artifacts")) violations.push("retention_report_missing_core_local_artifacts_root");
for (const row of roots) {
  if (row.over_total_budget === true) violations.push(`root_over_total_budget:${row.path}`);
  const oversize = Array.isArray(row.oversize_files) ? row.oversize_files : [];
  if (oversize.length > 0) violations.push(`oversize_artifact_files:${row.path}:${oversize.length}`);
  const rawMarkers = Array.isArray(row.raw_marker_files) ? row.raw_marker_files : [];
  if (rawMarkers.length > 0) violations.push(`raw_evidence_marker_files:${row.path}:${rawMarkers.length}`);
  if (typeof row.cleanup_candidate_count !== "number") violations.push(`cleanup_candidate_count_missing:${row.path}`);
  if (typeof row.canonical_latest_ref_count !== "number") violations.push(`canonical_latest_ref_count_missing:${row.path}`);
}
const latestRefPolicy = policy?.latest_ref_policy && typeof policy.latest_ref_policy === "object"
  ? policy.latest_ref_policy as Json
  : {};
const cleanupBudget = policy?.cleanup_candidate_budget && typeof policy.cleanup_candidate_budget === "object"
  ? policy.cleanup_candidate_budget as Json
  : {};
const actions = policy?.actions && typeof policy.actions === "object" ? policy.actions as Json : {};
const applyPolicy = policy?.apply && typeof policy.apply === "object" ? policy.apply as Json : {};
const applyScript = String(applyPolicy.script_path || "");
if (actions.apply_script_required === true) {
  if (!applyScript) violations.push("retention_apply_script_path_missing");
  else if (!fs.existsSync(path.join(root, applyScript))) violations.push("retention_apply_script_missing");
}
if (actions.delete_automatically === true) violations.push("retention_policy_must_not_delete_automatically");
if (actions.live_cleanup_requires_ack !== true) violations.push("retention_cleanup_ack_not_required");
if (typeof report?.cleanup_candidate_total !== "number") violations.push("retention_report_missing_cleanup_candidate_total");
if (typeof report?.cleanup_candidate_bytes !== "number") violations.push("retention_report_missing_cleanup_candidate_bytes");
const cleanupPressure = report?.cleanup_candidate_pressure && typeof report.cleanup_candidate_pressure === "object"
  ? report.cleanup_candidate_pressure as Json
  : {};
const cleanupPressureSeverity = String(cleanupPressure.severity || "");
if (!cleanupPressureSeverity) violations.push("retention_report_missing_cleanup_candidate_pressure");
if (cleanupPressureSeverity === "red" && cleanupBudget.fail_on_red_candidate_pressure === true) {
  violations.push("cleanup_candidate_pressure_red");
}
if (cleanupPressureSeverity === "yellow" && cleanupBudget.yellow_candidate_pressure_is_actionable === true) {
  advisories.push("cleanup_candidate_pressure_yellow");
}
const enforcement = report?.enforcement && typeof report.enforcement === "object" ? report.enforcement as Json : {};
if (String(enforcement.apply_script_path || "") !== applyScript) violations.push("retention_report_apply_script_mismatch");
if (enforcement.live_cleanup_requires_ack !== true) violations.push("retention_report_missing_cleanup_ack_policy");
const latestRefPath = path.join(root, String(report?.latest_ref_index_path || latestRefPolicy.index_path || ""));
const latestRefIndex = readJson(latestRefPath);
if (latestRefPolicy.emit_index !== false) {
  if (!latestRefIndex) violations.push("missing_local_artifact_latest_ref_index");
  if (latestRefIndex && String(latestRefIndex.source_domain || "") !== String(latestRefPolicy.index_owner_domain || "observability")) {
    violations.push("local_artifact_latest_ref_index_wrong_source_domain");
  }
  if (latestRefIndex && typeof latestRefIndex.trace_id !== "string") violations.push("local_artifact_latest_ref_index_missing_trace_id");
  if (latestRefIndex && typeof latestRefIndex.span_id !== "string") violations.push("local_artifact_latest_ref_index_missing_span_id");
  if (latestRefIndex && !Array.isArray(latestRefIndex.refs)) violations.push("local_artifact_latest_ref_index_missing_refs");
  if (latestRefIndex && typeof latestRefIndex.ref_count !== "number") violations.push("local_artifact_latest_ref_index_missing_count");
  const refs = latestRefIndex && Array.isArray(latestRefIndex.refs) ? latestRefIndex.refs as Json[] : [];
  const missingRootRefs = refs.filter((row) => typeof row.root_path !== "string" || !row.root_path).length;
  const missingCanonicalFields = refs.filter((row) => !Object.prototype.hasOwnProperty.call(row, "canonical_ref")).length;
  const missingNewestFields = refs.filter((row) => typeof row.latest_path !== "string" || !row.latest_path).length;
  if (missingRootRefs > 0 && latestRefPolicy.include_root_path === true) {
    violations.push(`local_artifact_latest_ref_index_missing_root_path:${missingRootRefs}`);
  }
  if (missingCanonicalFields > 0 && latestRefPolicy.include_canonical_ref === true) {
    violations.push(`local_artifact_latest_ref_index_missing_canonical_ref_field:${missingCanonicalFields}`);
  }
  if (missingNewestFields > 0) violations.push(`local_artifact_latest_ref_index_missing_latest_path:${missingNewestFields}`);
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention-guard`,
  span_id: `span:validation:${new Date().toISOString()}:assurance-artifact-retention-guard`,
  parent_span_id: report?.span_id || report?.trace_id || null,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "assurance_artifact_retention_guard",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  report_path: path.relative(root, reportPath),
  latest_ref_index_path: path.relative(root, latestRefPath),
  cleanup_candidate_total: Number(report?.cleanup_candidate_total || 0),
  cleanup_candidate_bytes: Number(report?.cleanup_candidate_bytes || 0),
  cleanup_candidate_pressure: cleanupPressure,
  advisories,
  violations,
};
const guardResultRel = String(policy?.guard_result_path || "core/local/artifacts/assurance_artifact_retention_guard_current.json");
const guardResultPath = path.join(root, guardResultRel);
fs.mkdirSync(path.dirname(guardResultPath), { recursive: true });
fs.writeFileSync(guardResultPath, `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
