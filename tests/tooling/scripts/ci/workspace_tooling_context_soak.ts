#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'workspace_tooling_context_soak');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');
const TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_WORKSPACE_TOOLING_SOAK_TIMEOUT_MS || '900000', 10) || 900_000,
);

type SoakLane = 'routing' | 'hints' | 'synthesis';

type SoakCase = {
  id: string;
  lane: SoakLane;
  test: string;
};

type SoakCaseResult = {
  id: string;
  lane: SoakLane;
  test: string;
  status: number;
  ok: boolean;
  duration_ms: number;
  timed_out: boolean;
  stdout_tail: string;
  stderr_tail: string;
};

type SoakReport = {
  type: 'workspace_tooling_context_soak_report';
  schema_version: 1;
  started_at: string;
  finished_at: string;
  ok: boolean;
  command: string;
  status: number;
  duration_ms: number;
  taxonomy: Record<string, unknown>;
  lane_pack: Record<string, unknown>;
  tests: SoakCaseResult[];
  stdout_tail: string;
  stderr_tail: string;
};

const CASES: SoakCase[] = [
  {
    id: 'routing_file_edit_classifies_to_task_route',
    lane: 'routing',
    test: 'workflow_decision_tree_v2_classifies_file_edits_as_task_route',
  },
  {
    id: 'routing_workspace_compare_not_forced_to_web',
    lane: 'routing',
    test: 'natural_web_intent_does_not_force_plain_workspace_peer_compare_into_web',
  },
  {
    id: 'hints_compare_clusters_workspace_and_web_tools',
    lane: 'hints',
    test: 'compare_workflow_hint_clusters_workspace_and_web_tools',
  },
  {
    id: 'synthesis_decomposes_workspace_and_web_evidence',
    lane: 'synthesis',
    test: 'compare_workflow_harness_decomposes_local_and_web_evidence_before_final_synthesis',
  },
];

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function cleanText(raw: unknown, maxLen = 3200): string {
  return String(raw ?? '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function writeJson(pathname: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(pathname), { recursive: true });
  fs.writeFileSync(pathname, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function runCargoTestWithTimeoutKill(testName: string): SoakCaseResult {
  const started = Date.now();
  const commandArgs = [
    'test',
    '-p',
    'protheus-ops-core',
    '--lib',
    testName,
    '--quiet',
    '--',
    '--nocapture',
  ];
  const run = spawnSync('cargo', commandArgs, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: TIMEOUT_MS,
    killSignal: 'SIGKILL',
  });
  const timeoutMessage = String(run.error?.message || '').toLowerCase();
  const timedOut =
    !!run.error && (timeoutMessage.includes('timed out') || timeoutMessage.includes('etimedout'));
  if (timedOut) {
    const pattern = `cargo test -p protheus-ops-core --lib ${testName}`;
    spawnSync('pkill', ['-TERM', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
    spawnSync('pkill', ['-KILL', '-f', pattern], { cwd: ROOT, stdio: 'ignore' });
  }
  const status = timedOut ? 124 : Number.isFinite(run.status) ? Number(run.status) : 1;
  const stdoutRaw = String(run.stdout || '');
  const stderrRaw = String(run.stderr || '');
  return {
    id: '',
    lane: 'routing',
    test: testName,
    status,
    ok: status === 0,
    duration_ms: Date.now() - started,
    timed_out: timedOut,
    stdout_tail: cleanText(stdoutRaw, 4_000),
    stderr_tail: cleanText(
      `${stderrRaw} ${timedOut ? `timeout_after_ms_${TIMEOUT_MS}` : ''}`.trim(),
      4_000,
    ),
  };
}

function laneSummary(results: SoakCaseResult[]): Record<string, unknown> {
  const lanes: SoakLane[] = ['routing', 'hints', 'synthesis'];
  const out: Record<string, unknown> = {};
  for (const lane of lanes) {
    const laneRows = results.filter((row) => row.lane === lane);
    const failed = laneRows.filter((row) => !row.ok);
    out[lane] = {
      total: laneRows.length,
      passed: laneRows.length - failed.length,
      failed: failed.length,
      failed_ids: failed.map((row) => row.id),
      ok: failed.length === 0,
    };
  }
  return out;
}

function caseOk(results: SoakCaseResult[], id: string): boolean {
  return results.find((row) => row.id === id)?.ok === true;
}

const startedAt = nowIso();
const startedMs = Date.now();
const results: SoakCaseResult[] = [];

for (const row of CASES) {
  const run = runCargoTestWithTimeoutKill(row.test);
  results.push({
    ...run,
    id: row.id,
    lane: row.lane,
  });
}

const allTestsPassed = results.every((row) => row.ok);
const lanePack = laneSummary(results);
const taxonomy = {
  family: 'workspace_file_tooling',
  total_cases: results.length,
  passed_cases: results.filter((row) => row.ok).length,
  failed_cases: results.filter((row) => !row.ok).length,
  failed_ids: results.filter((row) => !row.ok).map((row) => row.id),
  invariants: {
    file_edit_routes_to_task_lane: caseOk(results, 'routing_file_edit_classifies_to_task_route'),
    workspace_compare_not_forced_to_web: caseOk(results, 'routing_workspace_compare_not_forced_to_web'),
    compare_hint_cluster_includes_workspace_and_web: caseOk(
      results,
      'hints_compare_clusters_workspace_and_web_tools',
    ),
    workspace_web_decomposition_preserves_final_synthesis: caseOk(
      results,
      'synthesis_decomposes_workspace_and_web_evidence',
    ),
  },
};

const report: SoakReport = {
  type: 'workspace_tooling_context_soak_report',
  schema_version: 1,
  started_at: startedAt,
  finished_at: nowIso(),
  ok: allTestsPassed,
  command: 'cargo test -p protheus-ops-core --lib <workspace-workflow-test-name> -- --nocapture',
  status: allTestsPassed ? 0 : 1,
  duration_ms: Date.now() - startedMs,
  taxonomy,
  lane_pack: lanePack,
  tests: results,
  stdout_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stdout_tail}`).join('\n'),
    4_000,
  ),
  stderr_tail: cleanText(
    results.map((row) => `${row.id}: ${row.stderr_tail}`).join('\n'),
    4_000,
  ),
};

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `workspace_tooling_context_soak_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'workspace_tooling_context_soak_report_latest.json');
writeJson(stampedPath, report);
writeJson(latestPath, report);
writeJson(STATE_LATEST_PATH, report);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
