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
if (!report) violations.push("missing_assurance_artifact_retention_report");
if (report?.ok !== true) violations.push("retention_report_not_ok");
if (String(report?.source_domain || "") !== "validation") violations.push("retention_report_wrong_source_domain");
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
const latestRefPath = path.join(root, String(report?.latest_ref_index_path || latestRefPolicy.index_path || ""));
const latestRefIndex = readJson(latestRefPath);
if (latestRefPolicy.emit_index !== false) {
  if (!latestRefIndex) violations.push("missing_local_artifact_latest_ref_index");
  if (latestRefIndex && String(latestRefIndex.source_domain || "") !== String(latestRefPolicy.index_owner_domain || "observability")) {
    violations.push("local_artifact_latest_ref_index_wrong_source_domain");
  }
  if (latestRefIndex && !Array.isArray(latestRefIndex.refs)) violations.push("local_artifact_latest_ref_index_missing_refs");
  if (latestRefIndex && typeof latestRefIndex.ref_count !== "number") violations.push("local_artifact_latest_ref_index_missing_count");
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention-guard`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "assurance_artifact_retention_guard",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  report_path: path.relative(root, reportPath),
  latest_ref_index_path: path.relative(root, latestRefPath),
  violations,
};
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
