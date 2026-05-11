import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/regression/fixtures/gateway_idempotence/gateway_supervisor_authority_drift_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/gateway_supervisor_authority_drift_current.json"));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const violations: string[] = [];

for (const field of (policy.required_report_fields as string[]) || []) {
  if (!(field in report)) violations.push(`report_missing_${field}`);
}
if (!((policy.diagnostic_allowlist as string[]) || []).includes(String(report.diagnostic || ""))) {
  violations.push("report_diagnostic_not_allowlisted");
}
if (String(report.diagnostic || "") === "gateway_supervisor_authority_drift_detected") {
  const findings = Array.isArray(report.findings) ? report.findings : [];
  const nextActions = Array.isArray(report.next_actions) ? report.next_actions : [];
  if (findings.length === 0) violations.push("authority_drift_missing_clustered_findings");
  if (nextActions.length === 0) violations.push("authority_drift_missing_next_actions");
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:gateway-supervisor-authority-drift-guard`,
  source_domain: "validation",
  type: "gateway_supervisor_authority_drift_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  diagnostic: report.diagnostic || null,
  violation_count: violations.length,
  violations,
};

fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/gateway_supervisor_authority_drift_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
