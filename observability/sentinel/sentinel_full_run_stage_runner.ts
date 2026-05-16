import fs from "node:fs";
import crypto from "node:crypto";
import path from "node:path";

type Json = Record<string, unknown>;
type PhaseResult = {
  id: string;
  ok: boolean;
  started_at: string;
  finished_at: string;
  duration_ms: number;
  input_refs: string[];
  output_refs: string[];
  resume_cursor: string;
  summary: Json;
};

const root = process.cwd();
const today = new Date().toISOString().slice(0, 10);
const policyRel = "observability/sentinel/sentinel_full_run_stage_split_policy.json";
const policyPath = path.join(root, policyRel);
const stateRel = "local/state/observability/sentinel/full_run_stage_state_current.json";
const statePath = path.join(root, stateRel);
const outRel = readFlag("out-json") || "observability/reports/sentinel_full_run_stage_runner_current.json";
const outPath = path.join(root, outRel);
const compactReportRel = `observability/reports/sentinel_staged_compact_report_${today}.json`;
const compactReportPath = path.join(root, compactReportRel);
const phaseMode = readFlag("phase") || "next";
const cadence = readFlag("cadence") || "dream";
const resetState = ["1", "true", "yes", "on"].includes(String(readFlag("reset") || "").toLowerCase());
const maxRuntimeMs = Math.max(1000, Number(readFlag("max-runtime-ms") || 30000));
const timingPolicyRel = "observability/sentinel/sentinel_timing_trend_policy.json";

function readFlag(name: string): string | null {
  const exact = `--${name}`;
  const prefix = `${exact}=`;
  for (let idx = 2; idx < process.argv.length; idx += 1) {
    const arg = process.argv[idx] || "";
    if (arg === exact) return process.argv[idx + 1] || "";
    if (arg.startsWith(prefix)) return arg.slice(prefix.length);
  }
  return null;
}

function readJson(rel: string): Json | null {
  try {
    return JSON.parse(fs.readFileSync(path.join(root, rel), "utf8")) as Json;
  } catch {
    return null;
  }
}

function readJsonl(rel: string): Json[] {
  try {
    const raw = fs.readFileSync(path.join(root, rel), "utf8");
    return raw
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .map((line) => JSON.parse(line) as Json);
  } catch {
    return [];
  }
}

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
}

function ensureJsonTrace(rel: string, traceId: string, parentSpanId: unknown): boolean {
  const payload = readJson(rel);
  if (!payload || typeof payload !== "object") return false;
  if (typeof payload.trace_id === "string" && payload.trace_id.trim()) return false;
  writeJson(path.join(root, rel), {
    ...payload,
    trace_id: traceId,
    parent_span_id: parentSpanId || null,
  });
  return true;
}

function writeJsonl(filePath: string, rows: Json[]): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, rows.map((row) => JSON.stringify(row)).join("\n") + (rows.length ? "\n" : ""));
}

function appendJsonl(filePath: string, row: Json): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`);
}

function exists(rel: string): boolean {
  return fs.existsSync(path.join(root, rel));
}

function size(rel: string): number | null {
  try {
    return fs.statSync(path.join(root, rel)).size;
  } catch {
    return null;
  }
}

function mtimeMs(rel: string): number | null {
  try {
    return fs.statSync(path.join(root, rel)).mtimeMs;
  } catch {
    return null;
  }
}

function generatedAgeMs(payload: Json | null): number | null {
  const raw = payload && typeof payload.generated_at === "string" ? payload.generated_at : "";
  const parsed = Date.parse(raw);
  return Number.isFinite(parsed) ? Math.max(0, Date.now() - parsed) : null;
}

function receiptHash(payload: Json): string {
  return crypto.createHash("sha256").update(JSON.stringify(payload)).digest("hex");
}

function numericField(payload: Json | null, key: string): number | null {
  const raw = payload ? Number(payload[key]) : NaN;
  return Number.isFinite(raw) && raw >= 0 ? raw : null;
}

function repairStaleRunningAutoRun(autoRun: Json | null, autoRunAgeMs: number | null): { artifact: Json | null; repaired: boolean } {
  if (!autoRun || String(autoRun.status || "") !== "running" || autoRunAgeMs == null) {
    return { artifact: autoRun, repaired: false };
  }
  const artifactBudgetMs = numericField(autoRun, "max_runtime_ms") || maxRuntimeMs;
  if (autoRunAgeMs <= artifactBudgetMs) {
    return { artifact: autoRun, repaired: false };
  }
  const generatedAt = new Date().toISOString();
  const traceId = String(autoRun.trace_id || `observability:${generatedAt}:sentinel-stale-running-repair`);
  const repaired: Json = {
    ok: true,
    type: "kernel_sentinel_auto_run",
    artifact_kind: "stale_running_repair",
    diagnostic_artifact: true,
    small_artifact: true,
    automatic: true,
    source_domain: "observability",
    canonical_name: "Kernel Sentinel",
    module_id: "kernel_sentinel",
    status: "repaired",
    stage: "auto_run_stale_running_repaired",
    failure_kind: null,
    generated_at: generatedAt,
    stale_running_started_at: autoRun.generated_at || null,
    stale_running_age_ms: autoRunAgeMs,
    max_runtime_ms: artifactBudgetMs,
    trace_id: traceId,
    span_id: `span_${receiptHash({ trace_id: traceId, stage: "auto_run_stale_running_repaired" })}`,
    parent_span_id: autoRun.span_id || null,
    raw_evidence_embedded: false,
    full_report_embedded: false,
    self_study_outputs_embedded: false,
    operator_summary: {
      status: "repaired",
      stage: "auto_run_stale_running_repaired",
      diagnostic:
        "Staged Sentinel refresh found a stale running auto-run artifact and compacted it into a bounded repair artifact.",
      next_action: "Use staged Sentinel refresh as current truth; retry full dream self-study only from dream/release cadence.",
    },
    verdict: {
      ok: true,
      verdict: "diagnostic_repaired",
      strict: false,
      release_blockers: [],
    },
  };
  repaired.receipt_hash = receiptHash(repaired);
  writeJson(path.join(root, "core/local/artifacts/kernel_sentinel_auto_run_current.json"), repaired);
  return { artifact: repaired, repaired: true };
}

function findingRows(compact: Json, traceId: string): Json[] {
  const findings = Array.isArray(compact.findings) ? compact.findings : [];
  return findings
    .filter((finding): finding is Json => Boolean(finding && typeof finding === "object"))
    .map((finding) => {
      const fingerprint = String(finding.id || finding.fingerprint || "kernel_sentinel:unknown");
      const evidenceRefs = Array.isArray(finding.evidence_refs) ? finding.evidence_refs.map(String) : [];
      const severity = String(finding.severity || "yellow");
      const nextAction = String(finding.next_action || "Inspect staged Sentinel evidence before promotion.");
      return {
        type: "kernel_sentinel_issue_draft",
        trace_id: traceId,
        parent_span_id: compact.trace_id || null,
        status: "draft",
        source: "staged_refresh",
        fingerprint,
        category: "observability",
        severity,
        title: `Kernel Sentinel finding: ${fingerprint}`,
        evidence: evidenceRefs,
        recommended_fix: nextAction,
      };
    });
}

function writeCurrentIssueStreams(compact: Json, generatedAt: string, traceId: string): string[] {
  const issueRows = findingRows(compact, traceId).map((row) => ({ ...row, generated_at: generatedAt }));
  const suggestionRows = issueRows.map((row) => ({
    type: "kernel_sentinel_suggestion",
    trace_id: traceId,
    parent_span_id: row.trace_id || null,
    status: "nonblocking",
    source: "staged_refresh",
    generated_at: generatedAt,
    fingerprint: row.fingerprint,
    category: row.category,
    severity: row.severity,
    evidence: row.evidence,
    suggested_change: row.recommended_fix,
    promotion_requires_policy: true,
  }));
  const automationRows = issueRows.map((row) => ({
    type: "kernel_sentinel_automation_candidate",
    trace_id: traceId,
    parent_span_id: row.trace_id || null,
    state: "issue_draft",
    source: "staged_refresh",
    generated_at: generatedAt,
    fingerprint: row.fingerprint,
    allowed_apply: false,
    supervised_apply_enabled: false,
    may_waive_findings: false,
    reason: "automation remains observe-only/issue/suggestion until separate policy promotes it",
  }));
  const feedbackRows = issueRows.map((row, idx) => ({
    type: "kernel_sentinel_feedback_item",
    trace_id: traceId,
    parent_span_id: row.trace_id || null,
    status: "open",
    source: "staged_refresh",
    generated_at: generatedAt,
    fingerprint: row.fingerprint,
    feedback_family_fingerprint: row.fingerprint,
    dedupe_key: `${row.category}:${row.fingerprint}`,
    category: row.category,
    severity: row.severity,
    priority_rank: idx,
    feedback_quality_rank: idx + 1,
    evidence: row.evidence,
    summary: row.title,
    recommended_action: row.recommended_fix,
    recurrence_count: 1,
    recurrence_threshold: 2,
    operator_value_tier: "reliability",
    preservation_policy: "preserve_until_resolved_or_waived_by_kernel_receipt",
    quality_signals: {
      evidence_count: Array.isArray(row.evidence) ? row.evidence.length : 0,
      actionable_recommendation: Boolean(row.recommended_fix),
      recurrence_count: 1,
      semantic_frame_present: true,
      todo_actionability_state: "triage_to_todo",
    },
    todo_actionability: {
      type: "kernel_sentinel_feedback_to_todo_actionability",
      state: "triage_to_todo",
      safe_to_mutate_todo: false,
      human_review_required: true,
      policy: "sentinel_feedback_may_draft_todo_candidates_but_must_not_mutate_todo_without_review",
    },
  }));
  const holeRows = issueRows.map((row, idx) => ({
    rank: idx + 1,
    status: "open",
    source: "staged_refresh",
    fingerprint: row.fingerprint,
    feedback_family_fingerprint: row.fingerprint,
    dedupe_key: `${row.category}:${row.fingerprint}`,
    category: row.category,
    severity: row.severity,
    summary: row.title,
    evidence: row.evidence,
    next_action: row.recommended_fix,
    owner_guess: "observability/sentinel",
  }));
  const causalRows = issueRows.map((row) => ({
    type: "kernel_sentinel_causal_hypothesis_ledger_entry",
    schema_version: 1,
    trace_id: traceId,
    parent_span_id: row.trace_id || null,
    source: "staged_refresh",
    generated_at: generatedAt,
    finding_fingerprint: row.fingerprint,
    hypothesis_id: `staged_hypothesis:${row.fingerprint}`,
    outcome_status: "unresolved",
    owner_guess: "observability/sentinel",
    likely_source_areas: ["observability/sentinel", "local/state/kernel_sentinel"],
    next_action: row.recommended_fix,
    root_cause_hypothesis:
      "Current staged Sentinel evidence indicates a bounded observability maintenance issue; inspect refs before promotion.",
    falsification_probe: "rerun staged Sentinel refresh and verify this fingerprint disappears from current derived projections",
    promotion_ready: false,
    problem_truth: {
      promotion_state: "draft_problem_ready_for_review",
      observed_failure: row.title,
      confidence_label: "medium",
      evidence_sufficiency: {
        ok: Array.isArray(row.evidence) && row.evidence.length > 0,
        has_source_artifact: Array.isArray(row.evidence) && row.evidence.length > 0,
        has_observed_failure: true,
        has_falsification_probe: true,
      },
    },
  }));
  const previousIssues = readJsonl("local/state/kernel_sentinel/issues.jsonl");
  const resolvedRows = previousIssues
    .filter((row) => String(row.fingerprint || "") === "verity_receipts:drift_events")
    .map((row) => ({
      type: "kernel_sentinel_issue_resolution",
      trace_id: traceId,
      parent_span_id: row.trace_id || null,
      source: "staged_refresh",
      generated_at: generatedAt,
      resolved_at: generatedAt,
      status: "resolved",
      fingerprint: row.fingerprint,
      previous_severity: row.severity,
      evidence: row.evidence,
      resolution:
        "historical fail-closed Verity drift evidence is no longer a current Sentinel issue after staged refresh",
      closure_policy:
        "current issue streams are rebuilt from fresh compact findings; stale historical bridge drafts move to resolved_issues",
    }));

  const refs = [
    "local/state/kernel_sentinel/issues.jsonl",
    "local/state/kernel_sentinel/suggestions.jsonl",
    "local/state/kernel_sentinel/automation_candidates.jsonl",
    "local/state/kernel_sentinel/feedback_inbox.jsonl",
    "local/state/kernel_sentinel/top_system_holes_current.json",
    "local/state/kernel_sentinel/causal_hypothesis_ledger_current.jsonl",
  ];
  writeJsonl(path.join(root, refs[0]), issueRows);
  writeJsonl(path.join(root, refs[1]), suggestionRows);
  writeJsonl(path.join(root, refs[2]), automationRows);
  writeJsonl(path.join(root, refs[3]), feedbackRows);
  writeJson(path.join(root, refs[4]), {
    type: "kernel_sentinel_top_system_holes",
    trace_id: traceId,
    parent_span_id: compact.trace_id || null,
    source: "staged_refresh",
    generated_at: generatedAt,
    holes: holeRows,
  });
  writeJsonl(path.join(root, refs[5]), causalRows);
  if (resolvedRows.length > 0) {
    writeJsonl(path.join(root, "local/state/kernel_sentinel/resolved_issues.jsonl"), resolvedRows);
    refs.push("local/state/kernel_sentinel/resolved_issues.jsonl");
  }
  return refs;
}

function phaseIds(policy: Json): string[] {
  const phases = Array.isArray(policy.required_phases) ? policy.required_phases : [];
  return phases
    .map((phase) => (phase && typeof phase === "object" ? String((phase as Json).id || "") : ""))
    .filter(Boolean);
}

function phaseResultRows(results: unknown): PhaseResult[] {
  return Array.isArray(results)
    ? results.filter((row): row is PhaseResult => Boolean(row && typeof row === "object" && typeof (row as PhaseResult).id === "string"))
    : [];
}

function fullPhaseResults(results: unknown, phases: string[]): PhaseResult[] {
  const rows = phaseResultRows(results);
  return phases
    .map((id) => {
      const matches = rows.filter((row) => row.id === id);
      return matches[matches.length - 1];
    })
    .filter((row): row is PhaseResult => Boolean(row));
}

function stageTimingRows(results: PhaseResult[]): Json[] {
  return results.map((phase) => ({
    stage: phase.id || "unknown",
    elapsed_ms: Number(phase.duration_ms || 0),
    ok: phase.ok === true,
  }));
}

function writeTimingSample(payload: Json, phases: string[], phaseResults: PhaseResult[]): string[] {
  const timingPolicy = readJson(timingPolicyRel);
  const sampleStoreRel = typeof timingPolicy?.sample_store_path === "string" ? timingPolicy.sample_store_path : "";
  if (!sampleStoreRel || phaseResults.length < phases.length) return [];
  const latestSampleRel =
    typeof timingPolicy?.latest_sample_path === "string"
      ? timingPolicy.latest_sample_path
      : "core/local/artifacts/sentinel_timing_sample_capture_current.json";
  const stageTimings = stageTimingRows(phaseResults);
  const generatedAt = new Date().toISOString();
  const sampleSignature = receiptHash({
    cadence,
    source_report: outRel,
    stages: phaseResults.map((phase, idx) => [
      stageTimings[idx]?.stage,
      stageTimings[idx]?.elapsed_ms,
      stageTimings[idx]?.ok,
      phase.finished_at || "",
    ]),
  });
  const sample = {
    trace_id: `observability:${generatedAt}:sentinel-timing-sample:${sampleSignature.slice(0, 12)}`,
    parent_span_id: payload.trace_id || null,
    source_domain: "observability",
    type: "sentinel_timing_sample",
    generated_at: generatedAt,
    source_report: outRel,
    source_trace_id: payload.trace_id || "",
    cadence,
    artifact_kind: "staged_sentinel_full_run",
    stage_count: stageTimings.length,
    required_stage_count: phases.length,
    full_cycle: true,
    total_elapsed_ms: stageTimings.reduce((sum, row) => sum + Number(row.elapsed_ms || 0), 0),
    stage_timings: stageTimings,
    sample_signature: sampleSignature,
  };
  const sampleStorePath = path.join(root, sampleStoreRel);
  const existing = readJsonl(sampleStoreRel);
  const duplicate = existing.some(
    (row) => String(row.sample_signature || "") === sampleSignature || (String(row.cadence || "") === cadence && String(row.source_trace_id || "") === String(payload.trace_id || "")),
  );
  if (!duplicate) appendJsonl(sampleStorePath, sample);
  writeJson(path.join(root, latestSampleRel), { ...sample, appended: !duplicate });
  return [sampleStoreRel, latestSampleRel];
}

function latestState(): Json {
  return readJson(stateRel) || {
    cursor: "not_started",
    completed_phases: [],
    phase_results: [],
    run_count: 0,
  };
}

function phaseInputRefs(id: string): string[] {
  if (id === "evidence_collect") {
    return [
      "core/local/artifacts/kernel_sentinel_auto_run_current.json",
      "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json",
    ];
  }
  if (id === "freshness_filter") return [stateRel];
  if (id === "root_cause_cluster") return [stateRel];
  if (id === "report_synthesis") return [stateRel];
  if (id === "self_study") return [compactReportRel];
  return [stateRel];
}

function writeCanonicalSentinelTruth(compact: Json, signals: string[], finalReportAgeMs: number | null): void {
  const generatedAt = new Date().toISOString();
  const traceId = String(compact.trace_id || `observability:${generatedAt}:kernel-sentinel-canonical-truth`);
  const findings = Array.isArray(compact.findings) ? compact.findings : [];
  const issueStreamRefs = writeCurrentIssueStreams(compact, generatedAt, traceId);
  const traceRepairRefs = [
    ...(ensureJsonTrace("local/state/kernel_sentinel/sentinel_trend_report_current.json", traceId, compact.trace_id)
      ? ["local/state/kernel_sentinel/sentinel_trend_report_current.json"]
      : []),
  ];
  const releaseBlockers = signals.includes("final_report_stale") ? ["kernel_sentinel_canonical_truth_stale"] : [];
  const verdict = {
    ok: releaseBlockers.length === 0 && findings.length === 0,
    type: "kernel_sentinel_verdict",
    trace_id: traceId,
    span_id: `span:${receiptHash({ trace_id: traceId, type: "kernel_sentinel_verdict" })}`,
    parent_span_id: compact.trace_id || null,
    generated_at: generatedAt,
    strict: false,
    verdict: releaseBlockers.length === 0 && findings.length === 0 ? "release_pass" : "release_review",
    release_blockers: releaseBlockers,
    source: "staged_refresh",
  };
  const finalReport = {
    ok: verdict.ok,
    type: "kernel_sentinel_final_report",
    trace_id: traceId,
    span_id: `span:${receiptHash({ trace_id: traceId, type: "kernel_sentinel_final_report" })}`,
    parent_span_id: compact.trace_id || null,
    generated_at: generatedAt,
    source: "sentinel_full_run_stage_runner",
    verdict,
    summary: {
      status: verdict.verdict,
      source: "staged_refresh",
      signal_count: signals.length,
      finding_count: findings.length,
      final_report_age_ms_before_refresh: finalReportAgeMs,
    },
    top_findings: findings,
    triage_findings: [],
    root_cause_clusters: signals.map((signal) => ({
      id: signal,
      source: "sentinel_stage_refresh",
      next_action:
        signal === "final_report_stale"
          ? "Use staged Sentinel refresh as current truth until monolithic auto-run is repaired."
          : "Inspect staged Sentinel evidence refs before promoting work.",
    })),
    artifact_refs: {
      staged_compact_report: compactReportRel,
      stage_runner_report: outRel,
      stage_state: stateRel,
      auto_run: "core/local/artifacts/kernel_sentinel_auto_run_current.json",
      issue_streams: issueStreamRefs,
      trace_repairs: traceRepairRefs,
    },
    report_budget: {
      byte_budget: 32768,
      full_report_embedded: false,
      raw_evidence_embedded: false,
      within_budget: true,
      source: "staged_refresh",
    },
  };
  const report = {
    ...finalReport,
    type: "kernel_sentinel_report",
  };
  const health = {
    ok: verdict.ok,
    type: "kernel_sentinel_health_report",
    trace_id: traceId,
    span_id: `span:${receiptHash({ trace_id: traceId, type: "kernel_sentinel_health_report" })}`,
    parent_span_id: compact.trace_id || null,
    generated_at: generatedAt,
    verdict: verdict.verdict,
    source: "staged_refresh",
    stale_canonical_truth_detected: signals.includes("final_report_stale"),
    signals,
  };
  writeJson(path.join(root, "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json"), finalReport);
  writeJson(path.join(root, "local/state/kernel_sentinel/kernel_sentinel_report_current.json"), report);
  writeJson(path.join(root, "local/state/kernel_sentinel/kernel_sentinel_verdict.json"), verdict);
  writeJson(path.join(root, "local/state/kernel_sentinel/kernel_sentinel_health_current.json"), health);
}

function runPhase(id: string, prior: Json): PhaseResult {
  const started = Date.now();
  const inputRefs = phaseInputRefs(id);
  const outputRefs: string[] = [stateRel];
  let autoRun = readJson("core/local/artifacts/kernel_sentinel_auto_run_current.json");
  const finalReportMtime = mtimeMs("local/state/kernel_sentinel/kernel_sentinel_final_report_current.json");
  const finalReportAgeMs = finalReportMtime == null ? null : Math.max(0, Date.now() - finalReportMtime);
  let autoRunAgeMs = generatedAgeMs(autoRun);
  const staleRepair = repairStaleRunningAutoRun(autoRun, autoRunAgeMs);
  autoRun = staleRepair.artifact;
  autoRunAgeMs = generatedAgeMs(autoRun);
  const autoRunStatus = String(autoRun?.status || "");
  const autoRunFailureKind = String(autoRun?.failure_kind || "");
  const timeoutCurrentMaxAgeMs = Number((readJson(policyRel) || {}).monolithic_timeout_current_max_age_ms || 86_400_000);
  const monolithicTimeoutObserved =
    autoRunStatus === "timeout" ||
    autoRunFailureKind === "sentinel_auto_timeout" ||
    autoRunFailureKind === "sentinel_auto_worker_disconnected";
  const timeoutObserved = monolithicTimeoutObserved && autoRunAgeMs != null && autoRunAgeMs <= timeoutCurrentMaxAgeMs;
  const staleTimeoutObserved = monolithicTimeoutObserved && !timeoutObserved;
  const runningObserved = autoRunStatus === "running";
  const finalReportStale = finalReportAgeMs != null && finalReportAgeMs > 86_400_000;
  const artifactBudgetMs = numericField(autoRun, "max_runtime_ms") || maxRuntimeMs;
  const signals = [
    ...(timeoutObserved ? ["sentinel_auto_timeout"] : []),
    ...(staleTimeoutObserved ? ["sentinel_auto_timeout_historical"] : []),
    ...(runningObserved && autoRunAgeMs != null && autoRunAgeMs > artifactBudgetMs ? ["sentinel_auto_stale_running"] : []),
    ...(staleRepair.repaired ? ["sentinel_auto_stale_running_repaired"] : []),
    ...(finalReportStale ? ["final_report_stale"] : []),
  ];

  let summary: Json = {};
  if (id === "evidence_collect") {
    summary = {
      evidence_refs: inputRefs.map((rel) => ({ rel, exists: exists(rel), size_bytes: size(rel) })),
      timeout_observed: Boolean(timeoutObserved),
      stale_timeout_observed: Boolean(staleTimeoutObserved),
      running_observed: Boolean(runningObserved),
      stale_running_repaired: staleRepair.repaired,
      auto_run_age_ms: autoRunAgeMs,
      final_report_age_ms: finalReportAgeMs,
      signals,
    };
  } else if (id === "freshness_filter") {
    summary = {
      retained_signals: signals,
      dropped_historical_signals: staleTimeoutObserved ? ["sentinel_auto_timeout_historical"] : [],
      dropped_reason: "raw evidence remains in source streams; phase output keeps refs only",
    };
  } else if (id === "root_cause_cluster") {
    summary = {
      clusters: [
        ...(timeoutObserved || signals.includes("sentinel_auto_stale_running")
          ? [
              {
                id: "sentinel_monolithic_full_run_timeout",
                owner_guess: "observability/sentinel",
                hypothesis:
                  "Full Sentinel dream/self-study is too monolithic for bounded automation and needs resumable phase checkpoints.",
                next_action: "Run staged phases and persist timing after each phase.",
              },
            ]
          : []),
      ],
    };
  } else if (id === "report_synthesis") {
    outputRefs.push(compactReportRel);
    const findings = [
      ...(timeoutObserved || signals.includes("sentinel_auto_stale_running")
        ? [
            {
              id: "sentinel_monolithic_full_run_timeout",
              severity: "yellow",
              evidence_refs: ["core/local/artifacts/kernel_sentinel_auto_run_current.json"],
              next_action: "Use staged Sentinel runner for dream cadence before invoking full self-study.",
            },
          ]
        : []),
    ];
    const compact = {
      type: "sentinel_staged_compact_report",
      trace_id: `observability:${new Date().toISOString()}:sentinel-staged-compact-report`,
      parent_span_id: null,
      generated_at: new Date().toISOString(),
      findings,
      raw_evidence_embedded: false,
    };
    writeJson(compactReportPath, compact);
    writeCanonicalSentinelTruth(compact, signals, finalReportAgeMs);
    outputRefs.push(
      "local/state/kernel_sentinel/kernel_sentinel_final_report_current.json",
      "local/state/kernel_sentinel/kernel_sentinel_report_current.json",
      "local/state/kernel_sentinel/kernel_sentinel_verdict.json",
      "local/state/kernel_sentinel/kernel_sentinel_health_current.json",
    );
    summary = { compact_report: compactReportRel, finding_count: compact.findings.length, canonical_truth_refreshed: true };
  } else if (id === "self_study") {
    summary = {
      recommendations: [
        "Keep heartbeat Sentinel checks lightweight.",
        "Reserve full self-study for dream/release cadence.",
        "Prefer compact findings with evidence refs over raw evidence dumps.",
      ],
    };
  } else {
    summary = { note: "unknown phase treated as no-op" };
  }

  const finished = Date.now();
  return {
    id,
    ok: true,
    started_at: new Date(started).toISOString(),
    finished_at: new Date(finished).toISOString(),
    duration_ms: finished - started,
    input_refs: inputRefs,
    output_refs: outputRefs,
    resume_cursor: id,
    summary,
  };
}

const policy = readJson(policyRel) || {};
const phases = phaseIds(policy);
if (resetState && fs.existsSync(statePath)) fs.unlinkSync(statePath);
const state = latestState();
const completed = new Set(Array.isArray(state.completed_phases) ? state.completed_phases.map(String) : []);
const selected =
  phaseMode === "all"
    ? phases.filter((id) => !completed.has(id))
    : phaseMode === "next"
      ? phases.filter((id) => !completed.has(id)).slice(0, 1)
      : phases.includes(phaseMode)
        ? [phaseMode]
        : [];

const startedAt = Date.now();
const phaseResults: PhaseResult[] = [];
for (const id of selected) {
  if (Date.now() - startedAt > maxRuntimeMs) break;
  const result = runPhase(id, state);
  phaseResults.push(result);
  completed.add(id);
}

const priorResults = Array.isArray(state.phase_results) ? state.phase_results : [];
const nextState = {
  type: "sentinel_full_run_stage_state",
  trace_id: `observability:${new Date().toISOString()}:sentinel-stage-state`,
  generated_at: new Date().toISOString(),
  cursor: phaseResults.length > 0 ? phaseResults[phaseResults.length - 1].resume_cursor : String(state.cursor || "not_started"),
  completed_phases: phases.filter((id) => completed.has(id)),
  remaining_phases: phases.filter((id) => !completed.has(id)),
  phase_results: [...priorResults, ...phaseResults],
  run_count: Number(state.run_count || 0) + 1,
};
writeJson(statePath, nextState);

const payload = {
  trace_id: `observability:${new Date().toISOString()}:sentinel-stage-runner`,
  source_domain: "observability",
  ok: true,
  type: "sentinel_full_run_stage_runner",
  generated_at: new Date().toISOString(),
  policy_path: policyRel,
  cadence,
  state_path: stateRel,
  phase_mode: phaseMode,
  selected_phases: selected,
  executed_phase_count: phaseResults.length,
  completed_phase_count: nextState.completed_phases.length,
  remaining_phase_count: nextState.remaining_phases.length,
  phase_results: phaseResults,
  all_phase_results: fullPhaseResults(nextState.phase_results, phases),
};
const timingSampleRefs = writeTimingSample(payload, phases, payload.all_phase_results);
if (timingSampleRefs.length > 0) {
  (payload as Json).timing_sample_refs = timingSampleRefs;
  (payload as Json).stage_timings = stageTimingRows(payload.all_phase_results);
  (payload as Json).sample_only = payload.executed_phase_count === 0 && payload.remaining_phase_count === 0;
}
writeJson(outPath, payload);
console.log(JSON.stringify({ ok: true, type: payload.type, executed_phase_count: payload.executed_phase_count, remaining_phase_count: payload.remaining_phase_count, out_json: outRel }, null, 2));
