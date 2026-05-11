#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0

import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const repoRoot = process.cwd();
const strict = process.argv.includes("--strict=1") || process.argv.includes("--strict");
const maxHistoricalAgeMs = 24 * 60 * 60 * 1000;
const targetFingerprint = "verity_receipts:drift_events";
const artifactRel = "core/local/artifacts/kernel_sentinel_verity_receipt_drift_closure_guard_current.json";

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(repoRoot, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function readJsonl(rel: string): Json[] {
  try {
    return fs
      .readFileSync(path.join(repoRoot, rel), "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line) as Json);
  } catch {
    return [];
  }
}

function writeJson(rel: string, payload: unknown): void {
  const outPath = path.join(repoRoot, rel);
  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
}

function isOpenStatus(row: Json): boolean {
  const status = String(row.status || "").toLowerCase();
  return !["closed", "resolved", "waived", "superseded"].includes(status);
}

function staleHistoricalVerityRow(row: Json): boolean {
  if (String(row.type || "") !== "verity_drift_violation") return false;
  const tsMs = Number(row.ts_ms);
  return Number.isFinite(tsMs) && Date.now() - tsMs > maxHistoricalAgeMs;
}

function failClosedVerityRow(row: Json): boolean {
  const directReceipt = row.validation_receipt && typeof row.validation_receipt === "object"
    ? (row.validation_receipt as Json)
    : null;
  const nestedDetails = row.details && typeof row.details === "object" ? (row.details as Json) : null;
  const nestedReceipt =
    nestedDetails?.validation_receipt && typeof nestedDetails.validation_receipt === "object"
      ? (nestedDetails.validation_receipt as Json)
      : null;
  return directReceipt?.fail_closed === true || nestedReceipt?.fail_closed === true;
}

function finalReportMentionsOpenFinding(report: Json | null): boolean {
  if (!report) return false;
  const candidateArrays = [report.top_findings, report.triage_findings, report.findings];
  return candidateArrays.some((value) =>
    Array.isArray(value)
      ? value.some((row) => {
          if (!row || typeof row !== "object") return false;
          const item = row as Json;
          return String(item.id || item.fingerprint || "") === targetFingerprint;
        })
      : false,
  );
}

const driftRows = readJsonl("local/state/ops/verity/drift_events.jsonl");
const issueRows = readJsonl("local/state/kernel_sentinel/issues.jsonl");
const resolvedRows = readJsonl("local/state/kernel_sentinel/resolved_issues.jsonl");
const finalReport = readJson("local/state/kernel_sentinel/kernel_sentinel_final_report_current.json");

const currentOpenIssueRows = issueRows.filter(
  (row) => String(row.fingerprint || "") === targetFingerprint && isOpenStatus(row),
);
const resolvedTargetRows = resolvedRows.filter((row) => String(row.fingerprint || "") === targetFingerprint);
const nonHistoricalDriftRows = driftRows.filter(
  (row) => !staleHistoricalVerityRow(row) || !failClosedVerityRow(row),
);
const reportMentionsCurrentFinding = finalReportMentionsOpenFinding(finalReport);

const failures: string[] = [];
if (nonHistoricalDriftRows.length > 0) {
  failures.push("verity_drift_rows_are_not_all_stale_fail_closed");
}
if (reportMentionsCurrentFinding) {
  failures.push("canonical_final_report_still_mentions_verity_drift_as_current_finding");
}
if (currentOpenIssueRows.length > 0 && resolvedTargetRows.length === 0) {
  failures.push("stale_verity_drift_issue_stream_has_open_issue_without_resolution");
}

const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-verity-receipt-drift-closure`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: finalReport?.trace_id || null,
  source_domain: "observability",
  ok: failures.length === 0,
  type: "kernel_sentinel_verity_receipt_drift_closure_guard",
  generated_at: new Date().toISOString(),
  strict,
  target_fingerprint: targetFingerprint,
  drift_row_count: driftRows.length,
  non_historical_drift_row_count: nonHistoricalDriftRows.length,
  current_open_issue_count: currentOpenIssueRows.length,
  resolved_issue_count: resolvedTargetRows.length,
  final_report_mentions_current_finding: reportMentionsCurrentFinding,
  failures,
  policy: {
    stale_fail_closed_verity_drift_is_historical_evidence: true,
    current_issue_stream_must_not_keep_historical_drift_open: true,
    canonical_report_must_not_promote_historical_drift_as_current_finding: true,
  },
};

writeJson(artifactRel, payload);
console.log(JSON.stringify(payload, null, 2));

if (strict && failures.length > 0) {
  process.exit(1);
}
