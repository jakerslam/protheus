#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'web_tooling_context_soak');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');
const TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_WEB_TOOLING_SOAK_TIMEOUT_MS || '900000', 10) || 900_000,
);

type SoakCase = {
  id: string;
  lane: 'soak' | 'replay';
  test: string;
  taxonomyMarker?: string;
};

type SoakCaseResult = {
  id: string;
  lane: 'soak' | 'replay';
  test: string;
  status: number;
  ok: boolean;
  duration_ms: number;
  timed_out: boolean;
  stdout_tail: string;
  stderr_tail: string;
};

type SoakReport = {
  type: 'web_tooling_context_soak_report';
  schema_version: 2;
  started_at: string;
  finished_at: string;
  ok: boolean;
  command: string;
  status: number;
  duration_ms: number;
  taxonomy: Record<string, unknown>;
  replay_pack: Record<string, unknown>;
  tests: SoakCaseResult[];
  stdout_tail: string;
  stderr_tail: string;
};

const CASES: SoakCase[] = [
  {
    id: 'context_soak_32_turns',
    lane: 'soak',
    test: 'workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures',
    taxonomyMarker: 'WEB_TOOLING_CONTEXT_SOAK_TAXONOMY=',
  },
  {
    id: 'replay_web_failure_final_contract',
    lane: 'replay',
    test: 'workflow_web_tool_failure_still_returns_final_user_response',
  },
  {
    id: 'replay_speculative_blocker_guard',
    lane: 'replay',
    test: 'workflow_repair_does_not_resurrect_prior_speculative_web_blocker_copy',
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

function parseTaxonomy(stdout: string, marker: string): Record<string, unknown> {
  const line = stdout
    .split('\n')
    .find((row) => row.trim().startsWith(marker));
  if (!line) {
    return {
      parse_error: 'taxonomy_marker_missing',
    };
  }
  const raw = line.trim().slice(marker.length);
  try {
    return JSON.parse(raw) as Record<string, unknown>;
  } catch {
    return {
      parse_error: 'taxonomy_json_parse_failed',
      raw: cleanText(raw, 1200),
    };
  }
}

function runCargoTestWithTimeoutKill(testName: string): {
  result: SoakCaseResult;
  stdoutRaw: string;
} {
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
    result: {
      id: '',
      lane: 'soak',
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
    },
    stdoutRaw,
  };
}

const startedAt = nowIso();
const startedMs = Date.now();
const results: SoakCaseResult[] = [];
let taxonomy: Record<string, unknown> = {
  parse_error: 'taxonomy_marker_missing',
};

for (const row of CASES) {
  const { result: run, stdoutRaw } = runCargoTestWithTimeoutKill(row.test);
  const result: SoakCaseResult = {
    ...run,
    id: row.id,
    lane: row.lane,
  };
  if (row.taxonomyMarker) {
    taxonomy = parseTaxonomy(stdoutRaw, row.taxonomyMarker);
  }
  results.push(result);
}

const replayRows = results.filter((row) => row.lane === 'replay');
const replayFailed = replayRows.filter((row) => !row.ok);
const replayPack = {
  total: replayRows.length,
  passed: replayRows.length - replayFailed.length,
  failed: replayFailed.length,
  failed_ids: replayFailed.map((row) => row.id),
};

const taxonomyError = cleanText(
  (taxonomy.parse_error as string) || '',
  120,
);
const allTestsPassed = results.every((row) => row.ok);
const ok = allTestsPassed && taxonomyError.length === 0;

const report: SoakReport = {
  type: 'web_tooling_context_soak_report',
  schema_version: 2,
  started_at: startedAt,
  finished_at: nowIso(),
  ok,
  command: 'cargo test -p protheus-ops-core --lib <workflow-test-name> -- --nocapture',
  status: ok ? 0 : 1,
  duration_ms: Date.now() - startedMs,
  taxonomy,
  replay_pack: replayPack,
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
const stampedPath = path.join(ARTIFACT_DIR, `web_tooling_context_soak_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'web_tooling_context_soak_report_latest.json');
writeJson(stampedPath, report);
writeJson(latestPath, report);
writeJson(STATE_LATEST_PATH, report);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(ok ? 0 : 1);
