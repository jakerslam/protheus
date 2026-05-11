import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/scorecards/repo_entropy_scorecard_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/repo_entropy_scorecard_current.json"));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const dimensions = Array.isArray(report.dimensions) ? report.dimensions as Json[] : [];
const violations: string[] = [];

for (const name of (policy.required_dimensions as string[]) || []) {
  if (!dimensions.some((row) => row.name === name)) violations.push(`missing_dimension_${name}`);
}
if (!report.trace_id) violations.push("missing_trace_id");
if (report.source_domain !== "validation") violations.push("wrong_source_domain");
for (const row of dimensions) {
  if ((row.severity === "red" || row.severity === "yellow") && (!Array.isArray(row.next_actions) || row.next_actions.length === 0)) {
    violations.push(`dimension_missing_next_actions_${row.name || "unknown"}`);
  }
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:repo-entropy-scorecard-guard`,
  source_domain: "validation",
  type: "repo_entropy_scorecard_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  scorecard_severity: report.severity || null,
  red_dimensions: report.red_dimensions || [],
  violations,
};

fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/repo_entropy_scorecard_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
