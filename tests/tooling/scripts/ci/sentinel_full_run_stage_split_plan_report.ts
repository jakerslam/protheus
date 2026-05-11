import fs from "node:fs";
import path from "node:path";

type JsonRecord = Record<string, unknown>;

const repoRoot = process.cwd();
const policyPath = path.join(repoRoot, "observability/sentinel/sentinel_full_run_stage_split_policy.json");
const autoArtifactPath = path.join(repoRoot, "core/local/artifacts/kernel_sentinel_auto_run_current.json");
const finalReportPath = path.join(repoRoot, "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json");
const reportPath = path.join(
  repoRoot,
  "observability/reports",
  `sentinel_full_run_stage_split_plan_${new Date().toISOString().slice(0, 10)}.json`,
);

function readJson(filePath: string): JsonRecord | null {
  try {
    return JSON.parse(fs.readFileSync(filePath, "utf8")) as JsonRecord;
  } catch {
    return null;
  }
}

function fileSize(filePath: string): number | null {
  try {
    return fs.statSync(filePath).size;
  } catch {
    return null;
  }
}

function phaseIds(policy: JsonRecord): string[] {
  const phases = Array.isArray(policy.required_phases) ? policy.required_phases : [];
  return phases
    .map((phase) => (phase && typeof phase === "object" ? String((phase as JsonRecord).id || "") : ""))
    .filter(Boolean);
}

const policy = readJson(policyPath) || {};
const autoArtifact = readJson(autoArtifactPath);
const finalReport = readJson(finalReportPath);
const phases = phaseIds(policy);
const timeoutObserved =
  autoArtifact?.status === "timeout" ||
  autoArtifact?.failure_kind === "sentinel_auto_timeout" ||
  (autoArtifact?.artifact_kind === "diagnostic" && autoArtifact?.ok === false);

const stageTimingCount = Array.isArray(autoArtifact?.stage_timings) ? autoArtifact.stage_timings.length : 0;
const report = {
  trace_id: `observability:${new Date().toISOString()}:sentinel-stage-split`,
  source_domain: "observability",
  type: "sentinel_full_run_stage_split_plan",
  generated_at: new Date().toISOString(),
  policy_path: "observability/sentinel/sentinel_full_run_stage_split_policy.json",
  input_artifacts: {
    auto_run_current: {
      path: "core/local/artifacts/kernel_sentinel_auto_run_current.json",
      exists: Boolean(autoArtifact),
      status: autoArtifact?.status || null,
      failure_kind: autoArtifact?.failure_kind || null,
      artifact_kind: autoArtifact?.artifact_kind || null,
      stage_timing_count: stageTimingCount,
      size_bytes: fileSize(autoArtifactPath),
    },
    final_report_current: {
      path: "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json",
      exists: Boolean(finalReport),
      size_bytes: fileSize(finalReportPath),
    },
  },
  timeout_observed: Boolean(timeoutObserved),
  required_phase_count: policy.required_phase_count || phases.length,
  planned_phase_count: phases.length,
  planned_phases: phases,
  plan_ready: phases.length >= Number(policy.required_phase_count || 0),
  root_cause_hypothesis:
    "Full Sentinel dream/self-study is currently too monolithic; bounded automatic maintenance needs resumable phases with partial timing artifacts before timeout.",
  implementation_targets: [
    "Add a Sentinel phase runner that can execute evidence_collect, freshness_filter, root_cause_cluster, report_synthesis, and self_study independently.",
    "Persist phase timings and resume cursors after each phase instead of only at final completion.",
    "Keep heartbeat Sentinel work lightweight and reserve full self-study for dream or release cadence.",
    "Keep final reports budgeted; raw evidence should remain in observability evidence streams."
  ],
  severity: timeoutObserved ? "yellow" : "white",
};

fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify({ ok: true, type: report.type, timeout_observed: report.timeout_observed, plan_ready: report.plan_ready, report_path: path.relative(repoRoot, reportPath) }, null, 2));
