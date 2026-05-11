import fs from "node:fs";
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
const maxRuntimeMs = Math.max(1000, Number(readFlag("max-runtime-ms") || 30000));

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

function writeJson(filePath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`);
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

function phaseIds(policy: Json): string[] {
  const phases = Array.isArray(policy.required_phases) ? policy.required_phases : [];
  return phases
    .map((phase) => (phase && typeof phase === "object" ? String((phase as Json).id || "") : ""))
    .filter(Boolean);
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

function runPhase(id: string, prior: Json): PhaseResult {
  const started = Date.now();
  const inputRefs = phaseInputRefs(id);
  const outputRefs: string[] = [stateRel];
  const autoRun = readJson("core/local/artifacts/kernel_sentinel_auto_run_current.json");
  const finalReportMtime = mtimeMs("local/state/kernel_sentinel/kernel_sentinel_final_report_current.json");
  const finalReportAgeMs = finalReportMtime == null ? null : Math.max(0, Date.now() - finalReportMtime);
  const timeoutObserved =
    autoRun?.status === "timeout" ||
    autoRun?.failure_kind === "sentinel_auto_timeout" ||
    (autoRun?.artifact_kind === "diagnostic" && autoRun?.ok === false);

  let summary: Json = {};
  if (id === "evidence_collect") {
    summary = {
      evidence_refs: inputRefs.map((rel) => ({ rel, exists: exists(rel), size_bytes: size(rel) })),
      timeout_observed: Boolean(timeoutObserved),
      final_report_age_ms: finalReportAgeMs,
    };
  } else if (id === "freshness_filter") {
    summary = {
      retained_signals: [
        ...(timeoutObserved ? ["sentinel_auto_timeout"] : []),
        ...(finalReportAgeMs != null && finalReportAgeMs > 86_400_000 ? ["final_report_stale"] : []),
      ],
      dropped_reason: "raw evidence remains in source streams; phase output keeps refs only",
    };
  } else if (id === "root_cause_cluster") {
    summary = {
      clusters: [
        ...(timeoutObserved
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
    const compact = {
      type: "sentinel_staged_compact_report",
      generated_at: new Date().toISOString(),
      findings: timeoutObserved
        ? [
            {
              id: "sentinel_monolithic_full_run_timeout",
              severity: "yellow",
              evidence_refs: ["core/local/artifacts/kernel_sentinel_auto_run_current.json"],
              next_action: "Use staged Sentinel runner for dream cadence before invoking full self-study.",
            },
          ]
        : [],
      raw_evidence_embedded: false,
    };
    writeJson(compactReportPath, compact);
    summary = { compact_report: compactReportRel, finding_count: compact.findings.length };
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
  state_path: stateRel,
  phase_mode: phaseMode,
  selected_phases: selected,
  executed_phase_count: phaseResults.length,
  completed_phase_count: nextState.completed_phases.length,
  remaining_phase_count: nextState.remaining_phases.length,
  phase_results: phaseResults,
};
writeJson(outPath, payload);
console.log(JSON.stringify({ ok: true, type: payload.type, executed_phase_count: payload.executed_phase_count, remaining_phase_count: payload.remaining_phase_count, out_json: outRel }, null, 2));
