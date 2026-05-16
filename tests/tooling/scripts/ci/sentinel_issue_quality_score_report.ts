import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type Candidate = {
  id: string;
  severity: string | null;
  evidence_refs: string[];
  owner_guess: string | null;
  root_cause_hypothesis: string | null;
  next_action: string | null;
  recurrence_count: number;
  trace_ids: string[];
  source_refs: string[];
  first_seen_source: string | null;
  last_seen_source: string | null;
  root_cause_cluster_key: string | null;
};

const root = process.cwd();
const policyRel = "validation/evals/sentinel_issue_quality_policy.json";
const policyPath = path.join(root, policyRel);
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const inputs = (policy.inputs || {}) as Json;
const weights = (policy.score_weights || {}) as Record<string, number>;
const reportRel = String(policy.report_path || "core/local/artifacts/sentinel_issue_quality_score_current.json");
const historyRel = String(policy.history_path || "local/state/kernel_sentinel/issue_quality_score_history.jsonl");

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function readJsonl(rel: string): Json[] {
  try {
    return fs
      .readFileSync(path.join(root, rel), "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line) as Json);
  } catch {
    return [];
  }
}

function writeJson(rel: string, payload: unknown): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function writeJsonl(rel: string, rows: Json[]): void {
  const filePath = path.join(root, rel);
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, rows.map((row) => JSON.stringify(row)).join("\n") + (rows.length ? "\n" : ""));
}

function exists(rel: string): boolean {
  try {
    return fs.statSync(path.join(root, rel)).isFile();
  } catch {
    return false;
  }
}

function mtimeAgeMs(rel: string): number | null {
  try {
    return Math.max(0, Date.now() - fs.statSync(path.join(root, rel)).mtimeMs);
  } catch {
    return null;
  }
}

function generatedAgeMs(payload: Json | null): number | null {
  const parsed = Date.parse(String(payload?.generated_at || ""));
  return Number.isFinite(parsed) ? Math.max(0, Date.now() - parsed) : null;
}

function compactReportRefs(): string[] {
  const explicit = Array.isArray(inputs.compact_reports) ? inputs.compact_reports.map(String) : [];
  const glob = String(inputs.compact_report_glob || "");
  const maxCount = Math.max(1, Number(inputs.max_compact_report_count || 5));
  const discovered: string[] = [];
  if (glob.endsWith("*.json")) {
    const dirRel = path.dirname(glob);
    const prefix = path.basename(glob).replace("*.json", "");
    const dir = path.join(root, dirRel);
    try {
      discovered.push(
        ...fs
          .readdirSync(dir)
          .filter((name) => name.startsWith(prefix) && name.endsWith(".json"))
          .map((name) => path.join(dirRel, name))
          .sort((a, b) => (mtimeAgeMs(a) ?? Number.MAX_SAFE_INTEGER) - (mtimeAgeMs(b) ?? Number.MAX_SAFE_INTEGER))
          .slice(0, maxCount),
      );
    } catch {
      // Static policy may point at a directory that does not exist in early bootstrap.
    }
  }
  const freshnessBudget = Number(policy.freshness_budget_ms || 604_800_000);
  return [...new Set([...discovered, ...explicit])]
    .filter(exists)
    .filter((rel) => {
      const payload = readJson(rel);
      const age = generatedAgeMs(payload) ?? mtimeAgeMs(rel);
      return age !== null && age <= freshnessBudget;
    })
    .slice(0, maxCount);
}

function stringValue(row: Json, keys: string[]): string | null {
  for (const key of keys) {
    const raw = row[key];
    if (typeof raw === "string" && raw.trim()) return raw.trim();
  }
  return null;
}

function evidenceValues(row: Json): string[] {
  const refs = [
    ...(Array.isArray(row.evidence_refs) ? row.evidence_refs : []),
    ...(Array.isArray(row.evidence) ? row.evidence : []),
    ...(Array.isArray(row.artifact_refs) ? row.artifact_refs : []),
  ]
    .map(String)
    .map((ref) => ref.trim())
    .filter(Boolean);
  return [...new Set(refs)];
}

function traceValues(row: Json): string[] {
  return [row.trace_id, row.source_trace_id, row.parent_span_id]
    .filter((value): value is string => typeof value === "string" && value.trim().length > 0)
    .map((value) => value.trim());
}

function upsert(map: Map<string, Candidate>, rawId: string, sourceRef: string): Candidate {
  const id = rawId.trim();
  const current = map.get(id);
  if (current) {
    current.last_seen_source = sourceRef;
    if (!current.source_refs.includes(sourceRef)) current.source_refs.push(sourceRef);
    return current;
  }
  const created: Candidate = {
    id,
    severity: null,
    evidence_refs: [],
    owner_guess: null,
    root_cause_hypothesis: null,
    next_action: null,
    recurrence_count: 0,
    trace_ids: [],
    source_refs: [sourceRef],
    first_seen_source: sourceRef,
    last_seen_source: sourceRef,
    root_cause_cluster_key: null,
  };
  map.set(id, created);
  return created;
}

function mergeCandidate(candidate: Candidate, row: Json, sourceRef: string): void {
  candidate.severity = stringValue(row, ["severity", "failure_level"]) || candidate.severity;
  candidate.owner_guess =
    stringValue(row, ["owner_guess", "component", "source_domain", "category"]) || candidate.owner_guess;
  candidate.root_cause_hypothesis =
    stringValue(row, ["root_cause_hypothesis", "hypothesis", "summary", "observed_failure"]) ||
    candidate.root_cause_hypothesis;
  candidate.next_action =
    stringValue(row, ["next_action", "recommended_fix", "recommended_action", "suggested_change"]) ||
    candidate.next_action;
  candidate.root_cause_cluster_key =
    stringValue(row, ["root_cause_cluster_key", "cluster_key", "feedback_family_fingerprint", "finding_fingerprint"]) ||
    candidate.root_cause_cluster_key;
  candidate.evidence_refs.push(...evidenceValues(row));
  candidate.trace_ids.push(...traceValues(row));
  candidate.evidence_refs = [...new Set(candidate.evidence_refs)];
  candidate.trace_ids = [...new Set(candidate.trace_ids)];
  candidate.recurrence_count += 1;
  if (!candidate.source_refs.includes(sourceRef)) candidate.source_refs.push(sourceRef);
  candidate.last_seen_source = sourceRef;
}

function inferOwner(candidate: Candidate): string {
  const haystack = [candidate.id, candidate.owner_guess || "", ...candidate.source_refs].join(" ").toLowerCase();
  if (haystack.includes("gateway")) return "gateways";
  if (haystack.includes("install") || haystack.includes("windows")) return "installer";
  if (haystack.includes("validation") || haystack.includes("eval") || haystack.includes("guard")) return "validation";
  if (haystack.includes("kernel") || haystack.includes("memory")) return "kernel";
  if (haystack.includes("trace") || haystack.includes("sentinel") || haystack.includes("collector")) {
    return "observability/sentinel";
  }
  return "observability/sentinel";
}

function inferRootCauseClusterKey(candidate: Candidate): string {
  if (candidate.root_cause_cluster_key) return candidate.root_cause_cluster_key;
  const haystack = [candidate.id, candidate.root_cause_hypothesis || "", candidate.next_action || "", ...candidate.source_refs]
    .join(" ")
    .toLowerCase();
  if (haystack.includes("monolithic") || haystack.includes("timeout") || haystack.includes("collector") || haystack.includes("stale")) {
    return "sentinel_cadence_and_feedback_freshness";
  }
  if (haystack.includes("trace")) return "sentinel_trace_integrity";
  if (haystack.includes("report") || haystack.includes("size") || haystack.includes("bounded")) return "sentinel_report_boundedness";
  if (haystack.includes("quality") || haystack.includes("promotion") || haystack.includes("todo")) return "sentinel_issue_quality_promotion";
  return `sentinel_${crypto.createHash("sha256").update(candidate.id).digest("hex").slice(0, 12)}`;
}

function candidateId(row: Json): string | null {
  return stringValue(row, [
    "id",
    "fingerprint",
    "finding_fingerprint",
    "feedback_family_fingerprint",
    "dedupe_key",
    "hypothesis_id",
    "cluster_key",
  ]);
}

function collectCompactReports(map: Map<string, Candidate>): string[] {
  const refs = compactReportRefs();
  for (const rel of refs) {
    const report = readJson(rel);
    const reportAge = generatedAgeMs(report) ?? mtimeAgeMs(rel);
    const monolithicTimeoutCurrentMaxAgeMs = Number(policy.monolithic_timeout_current_max_age_ms || 86_400_000);
    const findings = Array.isArray(report?.findings) ? (report.findings as Json[]) : [];
    for (const finding of findings) {
      const id = candidateId(finding);
      if (!id) continue;
      if (id === "sentinel_monolithic_full_run_timeout" && (reportAge === null || reportAge > monolithicTimeoutCurrentMaxAgeMs)) {
        continue;
      }
      mergeCandidate(upsert(map, id, rel), { ...finding, trace_id: report?.trace_id }, rel);
    }
  }
  return refs;
}

function collectStageRunnerClusters(map: Map<string, Candidate>): string | null {
  const rel = String(inputs.stage_runner_report || "");
  const stageRunner = readJson(rel);
  const phases = Array.isArray(stageRunner?.phase_results) ? (stageRunner.phase_results as Json[]) : [];
  for (const phase of phases) {
    const summary = (phase.summary || {}) as Json;
    const clusters = Array.isArray(summary.clusters) ? (summary.clusters as Json[]) : [];
    for (const cluster of clusters) {
      const id = candidateId(cluster);
      if (!id) continue;
      mergeCandidate(upsert(map, id, rel), { ...cluster, trace_id: stageRunner?.trace_id }, rel);
    }
  }
  return stageRunner ? rel : null;
}

function collectIssueStreams(map: Map<string, Candidate>): string[] {
  const refs = Array.isArray(inputs.issue_streams) ? inputs.issue_streams.map(String) : [];
  for (const rel of refs) {
    for (const row of readJsonl(rel)) {
      const id = candidateId(row);
      if (!id) continue;
      mergeCandidate(upsert(map, id, rel), row, rel);
    }
  }
  return refs.filter((rel) => readJsonl(rel).length > 0);
}

function collectProjectionReports(map: Map<string, Candidate>): string[] {
  const refs = Array.isArray(inputs.current_projection_reports) ? inputs.current_projection_reports.map(String) : [];
  for (const rel of refs) {
    const report = readJson(rel);
    if (!report) continue;
    const holes = Array.isArray(report.holes) ? (report.holes as Json[]) : [];
    const findings = Array.isArray(report.top_findings) ? (report.top_findings as Json[]) : [];
    for (const row of [...holes, ...findings]) {
      const id = candidateId(row);
      if (!id) continue;
      mergeCandidate(upsert(map, id, rel), { ...row, trace_id: report.trace_id }, rel);
    }
  }
  return refs.filter((rel) => readJson(rel) != null);
}

const candidates = new Map<string, Candidate>();
const compactRefs = collectCompactReports(candidates);
const stageRunnerRef = collectStageRunnerClusters(candidates);
const streamRefs = collectIssueStreams(candidates);
const projectionRefs = collectProjectionReports(candidates);
const supportingEvidence = Array.isArray(inputs.supporting_evidence) ? inputs.supporting_evidence.map(String) : [];
const generatedAt = new Date().toISOString();
const traceId = `validation:${generatedAt}:sentinel-issue-quality:${crypto
  .createHash("sha256")
  .update([...candidates.values()].map((row) => row.id).sort().join("|"))
  .digest("hex")
  .slice(0, 12)}`;
for (const candidate of candidates.values()) {
  if (!candidate.owner_guess) candidate.owner_guess = inferOwner(candidate);
  candidate.root_cause_cluster_key = inferRootCauseClusterKey(candidate);
  if (candidate.evidence_refs.length === 0) {
    candidate.evidence_refs.push(...supportingEvidence.filter(exists));
  }
  candidate.evidence_refs = [...new Set(candidate.evidence_refs)];
  if (candidate.trace_ids.length === 0) {
    candidate.trace_ids.push(traceId);
  }
}

function score(candidate: Candidate) {
  const evidenceExisting = candidate.evidence_refs.filter(exists);
  const freshestAge =
    evidenceExisting
      .map(mtimeAgeMs)
      .filter((age): age is number => typeof age === "number")
      .sort((a, b) => a - b)[0] ?? null;
  const freshnessBudget = Number(policy.freshness_budget_ms || 604_800_000);
  const dimensions = {
    evidence: evidenceExisting.length > 0 ? Number(weights.evidence || 0) : 0,
    freshness: freshestAge !== null && freshestAge <= freshnessBudget ? Number(weights.freshness || 0) : 0,
    owner_guess: candidate.owner_guess ? Number(weights.owner_guess || 0) : 0,
    root_cause_hypothesis: candidate.root_cause_hypothesis ? Number(weights.root_cause_hypothesis || 0) : 0,
    next_action: candidate.next_action ? Number(weights.next_action || 0) : 0,
    recurrence: candidate.recurrence_count > 1 ? Number(weights.recurrence || 0) : 0,
  };
  const failureReasons = [
    ...(dimensions.evidence > 0 ? [] : ["missing_existing_evidence_ref"]),
    ...(dimensions.freshness > 0 ? [] : ["stale_or_missing_fresh_evidence"]),
    ...(dimensions.owner_guess > 0 ? [] : ["missing_owner_guess"]),
    ...(dimensions.root_cause_hypothesis > 0 ? [] : ["missing_root_cause_hypothesis"]),
    ...(dimensions.next_action > 0 ? [] : ["missing_next_action"]),
    ...(dimensions.recurrence > 0 ? [] : ["not_recurrent_yet"]),
    ...(candidate.trace_ids.length > 0 ? [] : ["missing_trace_id"]),
  ];
  const total = Object.values(dimensions).reduce((sum, value) => sum + value, 0);
  const promotionThreshold = Number(policy.promotion_threshold || policy.minimum_promotion_ready_score || 80);
  const reviewThreshold = Number(policy.review_threshold || policy.minimum_human_review_score || 60);
  return {
    score: total,
    dimensions,
    evidence_existing_count: evidenceExisting.length,
    freshest_evidence_age_ms: freshestAge,
    quality_failure_reasons: failureReasons,
    promotion_state:
      total >= promotionThreshold && failureReasons.length === 0
        ? "promotion_ready"
        : total >= reviewThreshold
          ? "human_review"
          : "observation_only",
    confidence_band:
      total >= promotionThreshold && failureReasons.length === 0
        ? "issue_ready"
        : total >= reviewThreshold
          ? "human_review"
          : "observation_only",
  };
}

const scored = [...candidates.values()]
  .map((candidate) => {
    const quality = score(candidate);
    return {
      ...candidate,
      score: quality.score,
      score_dimensions: quality.dimensions,
      evidence_existing_count: quality.evidence_existing_count,
      freshest_evidence_age_ms: quality.freshest_evidence_age_ms,
      promotion_state: quality.promotion_state,
      confidence_band: quality.confidence_band,
      quality_failure_reasons: quality.quality_failure_reasons,
    };
  })
  .sort((a, b) => b.score - a.score || a.id.localeCompare(b.id));

const priorHistory = readJsonl(historyRel).slice(-Number(policy.history_retention_rows || 100));
const previousSummary = priorHistory[priorHistory.length - 1] || null;
const averageScore =
  scored.length === 0 ? 100 : Math.round(scored.reduce((sum, row) => sum + Number(row.score || 0), 0) / scored.length);
const historyRow = {
  trace_id: traceId,
  type: "sentinel_issue_quality_score_history_row",
  generated_at: generatedAt,
  candidate_count: scored.length,
  promotion_ready_count: scored.filter((row) => row.promotion_state === "promotion_ready").length,
  human_review_count: scored.filter((row) => row.promotion_state === "human_review").length,
  observation_only_count: scored.filter((row) => row.promotion_state === "observation_only").length,
  average_score: averageScore,
  average_score_delta:
    previousSummary && typeof previousSummary.average_score === "number"
      ? averageScore - Number(previousSummary.average_score)
      : null,
  recurring_candidate_count: scored.filter((row) => row.recurrence_count > 1).length,
};
const retainedHistory = [...priorHistory, historyRow].slice(-Number(policy.history_retention_rows || 100));
writeJsonl(historyRel, retainedHistory);

const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: null,
  source_domain: "validation",
  type: "sentinel_issue_quality_score",
  generated_at: generatedAt,
  policy_path: policyRel,
  ok: true,
  candidate_count: scored.length,
  promotion_ready_count: historyRow.promotion_ready_count,
  human_review_count: historyRow.human_review_count,
  observation_only_count: historyRow.observation_only_count,
  average_score: averageScore,
  quality_trend: {
    history_path: historyRel,
    history_rows: retainedHistory.length,
    previous_average_score: previousSummary?.average_score ?? null,
    average_score_delta: historyRow.average_score_delta,
    recurring_candidate_count: historyRow.recurring_candidate_count,
  },
  source_refs: {
    stage_runner_report: stageRunnerRef,
    compact_reports: compactRefs,
    issue_streams: streamRefs,
    projection_reports: projectionRefs,
    supporting_evidence: supportingEvidence.filter(exists),
  },
  scoring_contract: {
    promotion_threshold: Number(policy.promotion_threshold || 80),
    review_threshold: Number(policy.review_threshold || 60),
    required_dimensions: policy.required_score_dimensions || [],
    compact_ref_only: true,
  },
  candidates: scored,
};

writeJson(reportRel, report);
console.log(JSON.stringify(report, null, 2));
