#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0

import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const repoRoot = process.cwd();
const strict = process.argv.includes("--strict=1") || process.argv.includes("--strict");
const autoRunRel = "core/local/artifacts/kernel_sentinel_auto_run_current.json";
const finalReportRel = "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json";
const artifactRel = "core/local/artifacts/kernel_sentinel_stale_auto_run_repair_guard_current.json";

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(repoRoot, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function writeJson(rel: string, payload: unknown): void {
  const outPath = path.join(repoRoot, rel);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
}

function generatedAgeMs(payload: Json | null): number | null {
  const raw = typeof payload?.generated_at === "string" ? payload.generated_at : "";
  const parsed = Date.parse(raw);
  return Number.isFinite(parsed) ? Math.max(0, Date.now() - parsed) : null;
}

function numberField(payload: Json | null, key: string, fallback: number): number {
  const raw = Number(payload?.[key]);
  return Number.isFinite(raw) && raw >= 0 ? raw : fallback;
}

function reportMentionsTimeout(report: Json | null): boolean {
  const arrays = [report?.top_findings, report?.triage_findings, report?.findings];
  return arrays.some((value) =>
    Array.isArray(value)
      ? value.some((row) => {
          if (!row || typeof row !== "object") return false;
          const item = row as Json;
          return String(item.id || item.fingerprint || "") === "sentinel_monolithic_full_run_timeout";
        })
      : false,
  );
}

const autoRun = readJson(autoRunRel);
const finalReport = readJson(finalReportRel);
const ageMs = generatedAgeMs(autoRun);
const maxRuntimeMs = numberField(autoRun, "max_runtime_ms", 30_000);
const status = String(autoRun?.status || "");
const staleRunning = status === "running" && ageMs != null && ageMs > maxRuntimeMs;
const repaired = String(autoRun?.artifact_kind || "") === "stale_running_repair" && status === "repaired";
const timeoutFindingStillCurrent = reportMentionsTimeout(finalReport);

const failures: string[] = [];
if (staleRunning) failures.push("auto_run_artifact_still_stale_running");
if (timeoutFindingStillCurrent && repaired) failures.push("final_report_still_promotes_timeout_after_repair");

const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-stale-auto-run-repair`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: autoRun?.trace_id || finalReport?.trace_id || null,
  source_domain: "observability",
  ok: failures.length === 0,
  type: "kernel_sentinel_stale_auto_run_repair_guard",
  generated_at: new Date().toISOString(),
  strict,
  auto_run_status: status,
  auto_run_artifact_kind: autoRun?.artifact_kind || null,
  auto_run_age_ms: ageMs,
  max_runtime_ms: maxRuntimeMs,
  stale_running: staleRunning,
  repaired,
  final_report_mentions_timeout: timeoutFindingStillCurrent,
  failures,
  policy: {
    stale_running_auto_run_artifacts_must_be_compacted: true,
    repaired_stale_auto_run_must_not_remain_a_current_problem_finding: true,
  },
};

writeJson(artifactRel, payload);
console.log(JSON.stringify(payload, null, 2));

if (strict && failures.length > 0) {
  process.exit(1);
}
