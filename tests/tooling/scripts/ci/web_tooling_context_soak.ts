#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const TEST_NAME = 'workflow_web_tooling_context_soak_32_turns_reports_zero_terminal_failures';
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'web_tooling_context_soak');
const STATE_LATEST_PATH = path.join(STATE_DIR, 'latest.json');
const TIMEOUT_MS = Math.max(
  30_000,
  Number.parseInt(process.env.INFRING_WEB_TOOLING_SOAK_TIMEOUT_MS || '900000', 10) || 900_000,
);

type SoakReport = {
  type: 'web_tooling_context_soak_report';
  schema_version: 1;
  started_at: string;
  finished_at: string;
  ok: boolean;
  command: string;
  status: number;
  duration_ms: number;
  taxonomy: Record<string, unknown>;
  stdout_tail: string;
  stderr_tail: string;
};

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

function parseSoakTaxonomy(stdout: string): Record<string, unknown> {
  const marker = 'WEB_TOOLING_CONTEXT_SOAK_TAXONOMY=';
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

const startedAt = nowIso();
const startedMs = Date.now();
const commandArgs = [
  'test',
  '-p',
  'protheus-ops-core',
  '--lib',
  TEST_NAME,
  '--quiet',
  '--',
  '--nocapture',
];
const run = spawnSync('cargo', commandArgs, {
  cwd: ROOT,
  encoding: 'utf8',
  stdio: ['ignore', 'pipe', 'pipe'],
  timeout: TIMEOUT_MS,
});
const durationMs = Date.now() - startedMs;
const timeoutMessage = String(run.error?.message || '').toLowerCase();
const timedOut =
  !!run.error && (timeoutMessage.includes('timed out') || timeoutMessage.includes('etimedout'));
const status = timedOut ? 124 : Number.isFinite(run.status) ? Number(run.status) : 1;
const stdout = String(run.stdout || '');
const stderr = String(run.stderr || '');
const taxonomy = parseSoakTaxonomy(stdout);
const report: SoakReport = {
  type: 'web_tooling_context_soak_report',
  schema_version: 1,
  started_at: startedAt,
  finished_at: nowIso(),
  ok: status === 0,
  command: `cargo ${commandArgs.join(' ')}`,
  status,
  duration_ms: durationMs,
  taxonomy,
  stdout_tail: cleanText(stdout, 4_000),
  stderr_tail: cleanText(
    `${stderr} ${timedOut ? `timeout_after_ms_${TIMEOUT_MS}` : ''}`.trim(),
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
process.exit(report.ok ? 0 : 1);
