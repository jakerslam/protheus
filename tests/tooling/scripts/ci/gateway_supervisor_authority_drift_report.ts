import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const root = process.cwd();
const policyPath = path.join(root, "validation/regression/fixtures/gateway_idempotence/gateway_supervisor_authority_drift_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;

function readJson(filePath: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as Json;
  } catch {
    return null;
  }
}

const inputReportPath = path.join(root, String(policy.input_report_path || ""));
const inputReport = readJson(inputReportPath);
const evidence = [
  String(inputReport?.stdout_tail || ""),
  String(inputReport?.stderr_tail || ""),
].join("\n");
const forbiddenMarkers = (policy.forbidden_supervisor_markers as string[]) || [];
const advisoryMarkers = (policy.advisory_markers as string[]) || [];
const forbiddenHits = forbiddenMarkers.filter((marker) => evidence.includes(marker));
const advisoryHits = advisoryMarkers.filter((marker) => evidence.includes(marker));
const evidenceMissing = !inputReport || evidence.trim().length === 0;
const findings = [];

if (forbiddenHits.some((marker) => marker.includes("/local/workspace/shadow/"))) {
  findings.push({
    kind: "gateway_supervisor_shadow_workspace",
    severity: "yellow",
    evidence_marker: "/local/workspace/shadow/",
    owner_guess: "ops.gateway",
  });
}
if (forbiddenHits.some((marker) => marker.includes("ALLOW_STALE"))) {
  findings.push({
    kind: "gateway_supervisor_stale_binary_allowance",
    severity: "yellow",
    evidence_marker: "INFRING_*_ALLOW_STALE",
    owner_guess: "ops.gateway",
  });
}
if (advisoryHits.length > 0) {
  findings.push({
    kind: "gateway_supervisor_home_installed_binary_advisory",
    severity: "white",
    evidence_marker: advisoryHits[0],
    owner_guess: "ops.gateway",
  });
}

const diagnostic = evidenceMissing
  ? "gateway_supervisor_status_evidence_missing"
  : forbiddenHits.length > 0
    ? "gateway_supervisor_authority_drift_detected"
    : "gateway_supervisor_authority_current";
const nextActions = diagnostic === "gateway_supervisor_authority_drift_detected"
  ? [
      "Restart or reinstall the launchd supervisor from the canonical workspace root so status, watchdog logs, and dashboard service state use the same authority root.",
      "Remove stale-binary allowance environment variables from the persistent supervisor once the installed runtime binary is refreshed.",
      "Keep this as one authority-drift finding rather than separate symptom tickets for status output, stale binary flags, and shadow workspace logs.",
    ]
  : diagnostic === "gateway_supervisor_status_evidence_missing"
    ? [
        "Run the gateway status live replay before evaluating supervisor authority drift.",
      ]
    : [];
const traceId = `validation:${new Date().toISOString()}:gateway-supervisor-authority-drift`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: String(inputReport?.trace_id || "") || null,
  source_domain: "validation",
  type: "gateway_supervisor_authority_drift_report",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  input_report_path: String(policy.input_report_path || ""),
  diagnostic,
  severity: diagnostic === "gateway_supervisor_authority_current" ? "pass" : "yellow",
  root_cause_hypothesis: diagnostic === "gateway_supervisor_authority_drift_detected"
    ? "Gateway status is healthy, but the persistent supervisor still carries old authority context: shadow workspace root and stale-binary allowances. That can make future status/start/restart behavior diverge from source-authoritative development behavior."
    : diagnostic === "gateway_supervisor_status_evidence_missing"
      ? "Supervisor authority could not be evaluated because the gateway status replay evidence is missing."
      : "Gateway supervisor evidence does not show shadow workspace or stale-binary authority drift.",
  findings,
  forbidden_hits: forbiddenHits,
  advisory_hits: advisoryHits,
  next_actions: nextActions,
};

const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/gateway_supervisor_authority_drift_current.json"));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
