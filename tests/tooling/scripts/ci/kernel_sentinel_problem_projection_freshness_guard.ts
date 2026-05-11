#!/usr/bin/env node
// SPDX-License-Identifier: Apache-2.0

import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;

const repoRoot = process.cwd();
const strict = process.argv.includes("--strict=1") || process.argv.includes("--strict");
const artifactRel = "core/local/artifacts/kernel_sentinel_problem_projection_freshness_guard_current.json";

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

function activeIssueFingerprints(): Set<string> {
  const active = new Set<string>();
  for (const row of readJsonl("local/state/kernel_sentinel/issues.jsonl")) {
    const status = String(row.status || "").toLowerCase();
    if (["closed", "resolved", "waived", "superseded"].includes(status)) continue;
    const fingerprint = String(row.fingerprint || "");
    if (fingerprint) active.add(fingerprint);
  }
  return active;
}

function rowsWithFingerprint(rel: string, rows: Json[], key: string): Json[] {
  return rows
    .map((row) => ({
      projection: rel,
      fingerprint: String(row[key] || row.fingerprint || ""),
      row,
    }))
    .filter((row) => row.fingerprint);
}

const active = activeIssueFingerprints();
const topHoles = readJson("local/state/kernel_sentinel/top_system_holes_current.json");
const topHoleRows = Array.isArray(topHoles?.holes) ? (topHoles?.holes as Json[]) : [];
const projectionRows = [
  ...rowsWithFingerprint("local/state/kernel_sentinel/feedback_inbox.jsonl", readJsonl("local/state/kernel_sentinel/feedback_inbox.jsonl"), "fingerprint"),
  ...rowsWithFingerprint("local/state/kernel_sentinel/top_system_holes_current.json", topHoleRows, "fingerprint"),
  ...rowsWithFingerprint(
    "local/state/kernel_sentinel/causal_hypothesis_ledger_current.jsonl",
    readJsonl("local/state/kernel_sentinel/causal_hypothesis_ledger_current.jsonl"),
    "finding_fingerprint",
  ),
];

const staleProjectionRows = projectionRows.filter((row) => !active.has(row.fingerprint));
const failures = staleProjectionRows.length > 0 ? ["derived_problem_projection_contains_stale_fingerprints"] : [];
const traceId = `observability:${new Date().toISOString()}:kernel-sentinel-problem-projection-freshness`;
const payload = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: topHoles?.trace_id || null,
  source_domain: "observability",
  ok: failures.length === 0,
  type: "kernel_sentinel_problem_projection_freshness_guard",
  generated_at: new Date().toISOString(),
  strict,
  active_issue_fingerprints: Array.from(active).sort(),
  projection_fingerprint_count: projectionRows.length,
  stale_projection_count: staleProjectionRows.length,
  stale_projection_rows: staleProjectionRows.map((row) => ({
    projection: row.projection,
    fingerprint: row.fingerprint,
  })),
  failures,
  policy: {
    derived_problem_surfaces_must_be_rebuilt_from_current_issue_stream: true,
    resolved_or_historical_findings_must_not_remain_in_feedback_holes_or_causal_current_views: true,
  },
};

writeJson(artifactRel, payload);
console.log(JSON.stringify(payload, null, 2));

if (strict && failures.length > 0) {
  process.exit(1);
}
