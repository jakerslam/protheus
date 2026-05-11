import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/regression/fixtures/gateway_idempotence/gateway_status_launcher_drift_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/gateway_status_launcher_drift_current.json"));
const launcherPath = path.join(root, String(policy.launcher_path || ""));
const report = JSON.parse(fs.readFileSync(reportPath, "utf8")) as Json;
const launcherSource = fs.readFileSync(launcherPath, "utf8");
const violations: string[] = [];

for (const field of (policy.required_report_fields as string[]) || []) {
  if (!(field in report)) violations.push(`report_missing_${field}`);
}
if (!((policy.diagnostic_allowlist as string[]) || []).includes(String(report.diagnostic || ""))) {
  violations.push("report_diagnostic_not_allowlisted");
}
if (!launcherSource.includes("client/cli/npm/vendor/infring-ops")) violations.push("launcher_missing_vendor_candidate");
if (!launcherSource.includes("target/debug/infring-ops")) violations.push("launcher_missing_debug_candidate");
if (!launcherSource.includes("target/release/infring-ops")) violations.push("launcher_missing_release_candidate");
if (!launcherSource.includes("cargo run --quiet --manifest-path")) violations.push("launcher_missing_cargo_fallback");
if (launcherSource.includes("$HOME/.local") || launcherSource.includes(".infring/bin/infring-ops")) {
  violations.push("launcher_prefers_home_installed_binary");
}
if (String(report.diagnostic || "") !== "gateway_status_launcher_current") {
  const actions = Array.isArray(report.next_actions) ? report.next_actions : [];
  if (actions.length === 0) violations.push("non_pass_diagnostic_missing_next_actions");
}

const result = {
  trace_id: `validation:${new Date().toISOString()}:gateway-status-launcher-drift-guard`,
  source_domain: "validation",
  type: "gateway_status_launcher_drift_guard",
  generated_at: new Date().toISOString(),
  ok: violations.length === 0,
  report_path: path.relative(root, reportPath),
  diagnostic: report.diagnostic || null,
  selected_candidate: report.selected_candidate || null,
  violations,
};

fs.mkdirSync(path.join(root, "core/local/artifacts"), { recursive: true });
fs.writeFileSync(path.join(root, "core/local/artifacts/gateway_status_launcher_drift_guard_current.json"), `${JSON.stringify(result, null, 2)}\n`);
console.log(JSON.stringify(result, null, 2));
if (violations.length > 0) process.exitCode = 1;
