import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/conformance/contracts/guard_registry_ownership_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/guard_registry_ownership_current.json"));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const violations: string[] = [];

if (!report.trace_id) violations.push("missing_trace_id");
if (report.source_domain !== "validation") violations.push("wrong_source_domain");
if (!Array.isArray(report.rows)) violations.push("missing_guard_rows");
if (typeof report.guard_count !== "number") violations.push("missing_guard_count");
if (typeof report.missing_ownership_count !== "number") violations.push("missing_missing_ownership_count");
if (Number(report.missing_ownership_count || 0) > 0 && !Array.isArray(report.findings)) {
  violations.push("missing_findings_for_unowned_guards");
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:guard-registry-ownership-guard`,
  source_domain: "validation",
  type: "guard_registry_ownership_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  guard_count: report.guard_count || 0,
  missing_ownership_count: report.missing_ownership_count || 0,
  duplicate_family_count: report.duplicate_family_count || 0,
  violations,
};
fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/guard_registry_ownership_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
