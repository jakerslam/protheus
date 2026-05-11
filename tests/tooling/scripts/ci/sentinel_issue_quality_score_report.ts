import fs from "node:fs";
import path from "node:path";

type Json = Record<string, unknown>;
type Candidate = {
  id: string;
  severity?: string;
  evidence_refs: string[];
  owner_guess: string | null;
  root_cause_hypothesis: string | null;
  next_action: string | null;
  recurrence_count: number;
};

const root = process.cwd();
const policyPath = path.join(root, "validation/evals/sentinel_issue_quality_policy.json");
const policy = JSON.parse(fs.readFileSync(policyPath, "utf8")) as Json;
const inputs = policy.inputs as Json;
const weights = policy.score_weights as Record<string, number>;

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
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

function upsert(map: Map<string, Candidate>, id: string): Candidate {
  const current = map.get(id);
  if (current) return current;
  const created: Candidate = {
    id,
    evidence_refs: [],
    owner_guess: null,
    root_cause_hypothesis: null,
    next_action: null,
    recurrence_count: 0,
  };
  map.set(id, created);
  return created;
}

const candidates = new Map<string, Candidate>();
const compactReports = Array.isArray(inputs.compact_reports) ? inputs.compact_reports.map(String) : [];
for (const rel of compactReports) {
  const report = readJson(rel);
  const findings = Array.isArray(report?.findings) ? report.findings as Json[] : [];
  for (const finding of findings) {
    const id = String(finding.id || "");
    if (!id) continue;
    const candidate = upsert(candidates, id);
    candidate.severity = String(finding.severity || candidate.severity || "");
    candidate.evidence_refs.push(...(Array.isArray(finding.evidence_refs) ? finding.evidence_refs.map(String) : []));
    candidate.next_action = String(finding.next_action || candidate.next_action || "") || null;
    candidate.recurrence_count += 1;
  }
}

const stageRunner = readJson(String(inputs.stage_runner_report || ""));
const phases = Array.isArray(stageRunner?.phase_results) ? stageRunner.phase_results as Json[] : [];
for (const phase of phases) {
  const clusters = Array.isArray((phase.summary as Json | undefined)?.clusters) ? (phase.summary as Json).clusters as Json[] : [];
  for (const cluster of clusters) {
    const id = String(cluster.id || "");
    if (!id) continue;
    const candidate = upsert(candidates, id);
    candidate.owner_guess = String(cluster.owner_guess || candidate.owner_guess || "") || null;
    candidate.root_cause_hypothesis = String(cluster.hypothesis || cluster.root_cause_hypothesis || candidate.root_cause_hypothesis || "") || null;
    candidate.next_action = String(cluster.next_action || candidate.next_action || "") || null;
    candidate.recurrence_count += 1;
  }
}

const supportingEvidence = Array.isArray(inputs.supporting_evidence) ? inputs.supporting_evidence.map(String) : [];
for (const candidate of candidates.values()) {
  for (const rel of supportingEvidence) {
    if (candidate.evidence_refs.length === 0 && exists(rel)) candidate.evidence_refs.push(rel);
  }
  candidate.evidence_refs = [...new Set(candidate.evidence_refs)];
}

function score(candidate: Candidate) {
  const evidenceExisting = candidate.evidence_refs.filter(exists);
  const freshestAge = evidenceExisting
    .map(mtimeAgeMs)
    .filter((age): age is number => typeof age === "number")
    .sort((a, b) => a - b)[0] ?? null;
  const freshnessBudget = Number(policy.freshness_budget_ms || 604_800_000);
  const dimensions = {
    evidence: evidenceExisting.length > 0 ? weights.evidence : 0,
    freshness: freshestAge !== null && freshestAge <= freshnessBudget ? weights.freshness : 0,
    owner_guess: candidate.owner_guess ? weights.owner_guess : 0,
    root_cause_hypothesis: candidate.root_cause_hypothesis ? weights.root_cause_hypothesis : 0,
    next_action: candidate.next_action ? weights.next_action : 0,
    recurrence: candidate.recurrence_count > 1 ? weights.recurrence : 0,
  };
  const total = Object.values(dimensions).reduce((sum, value) => sum + value, 0);
  const promotionThreshold = Number(policy.promotion_threshold || 80);
  const reviewThreshold = Number(policy.review_threshold || 60);
  return {
    score: total,
    dimensions,
    evidence_existing_count: evidenceExisting.length,
    freshest_evidence_age_ms: freshestAge,
    promotion_state: total >= promotionThreshold ? "promotion_ready" : total >= reviewThreshold ? "human_review" : "observation_only",
  };
}

const scored = [...candidates.values()].map((candidate) => {
  const quality = score(candidate);
  return {
    ...candidate,
    score: quality.score,
    score_dimensions: quality.dimensions,
    evidence_existing_count: quality.evidence_existing_count,
    freshest_evidence_age_ms: quality.freshest_evidence_age_ms,
    promotion_state: quality.promotion_state,
  };
}).sort((a, b) => b.score - a.score || a.id.localeCompare(b.id));

const traceId = `validation:${new Date().toISOString()}:sentinel-issue-quality`;
const report = {
  trace_id: traceId,
  span_id: `span:${traceId}`,
  parent_span_id: String(stageRunner?.trace_id || "") || null,
  source_domain: "validation",
  type: "sentinel_issue_quality_score",
  generated_at: new Date().toISOString(),
  policy_path: path.relative(root, policyPath),
  ok: true,
  candidate_count: scored.length,
  promotion_ready_count: scored.filter((row) => row.promotion_state === "promotion_ready").length,
  human_review_count: scored.filter((row) => row.promotion_state === "human_review").length,
  observation_only_count: scored.filter((row) => row.promotion_state === "observation_only").length,
  candidates: scored,
};

const reportPath = path.join(root, String(policy.report_path || "core/local/artifacts/sentinel_issue_quality_score_current.json"));
fs.mkdirSync(path.dirname(reportPath), { recursive: true });
fs.writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
console.log(JSON.stringify(report, null, 2));
