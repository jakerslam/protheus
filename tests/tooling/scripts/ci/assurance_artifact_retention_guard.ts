import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const reportPath = path.join(
  root,
  "validation/reports",
  `assurance_artifact_retention_report_${new Date().toISOString().slice(0, 10)}.json`,
);

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as Json;
  } catch {
    return null;
  }
}

const report = readJson(reportPath);
const violations: string[] = [];
if (!report) violations.push("missing_assurance_artifact_retention_report");
if (report?.ok !== true) violations.push("retention_report_not_ok");
if (String(report?.source_domain || "") !== "validation") violations.push("retention_report_wrong_source_domain");
const roots = Array.isArray(report?.roots) ? report.roots as Json[] : [];
if (roots.length < 2) violations.push("retention_report_missing_validation_or_observability_root");
for (const row of roots) {
  if (row.over_total_budget === true) violations.push(`root_over_total_budget:${row.path}`);
  const oversize = Array.isArray(row.oversize_files) ? row.oversize_files : [];
  if (oversize.length > 0) violations.push(`oversize_artifact_files:${row.path}:${oversize.length}`);
  const rawMarkers = Array.isArray(row.raw_marker_files) ? row.raw_marker_files : [];
  if (rawMarkers.length > 0) violations.push(`raw_evidence_marker_files:${row.path}:${rawMarkers.length}`);
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:assurance-artifact-retention-guard`,
  source_domain: "validation",
  ok: violations.length === 0,
  type: "assurance_artifact_retention_guard",
  generated_at: new Date().toISOString(),
  report_path: path.relative(root, reportPath),
  violations,
};
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
