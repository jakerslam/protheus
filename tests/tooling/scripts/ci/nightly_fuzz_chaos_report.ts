#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');

function nowIso() {
  return new Date().toISOString();
}

function tsSlug(iso) {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function runStep(id, command, args, options = {}) {
  const startedAt = nowIso();
  const out = spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    ...options,
  });
  const endedAt = nowIso();
  return {
    id,
    command,
    args,
    status: Number.isFinite(out.status) ? out.status : 1,
    ok: (out.status ?? 1) === 0,
    started_at: startedAt,
    ended_at: endedAt,
    stdout: String(out.stdout || '').trim(),
    stderr: String(out.stderr || '').trim(),
  };
}

function hasFuzzTargets() {
  if (!fs.existsSync(path.join(ROOT, 'fuzz'))) return false;
  const probe = spawnSync(
    'bash',
    ['-lc', "find fuzz -maxdepth 3 -name '*.rs' | head -n 1"],
    { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] },
  );
  return String(probe.stdout || '').trim().length > 0;
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const startedAt = nowIso();
const report = {
  type: 'nightly_fuzz_chaos_report',
  ts: startedAt,
  cwd: ROOT,
  checks: [],
};

const fuzzPresent = hasFuzzTargets();
report.fuzz_targets_detected = fuzzPresent;

if (fuzzPresent) {
  report.checks.push(runStep('fuzz_target_list', 'cargo', ['fuzz', 'list']));
} else {
  report.checks.push({
    id: 'fuzz_target_list',
    command: 'cargo',
    args: ['fuzz', 'list'],
    status: 0,
    ok: true,
    skipped: true,
    reason: 'no_fuzz_targets_detected',
    started_at: nowIso(),
    ended_at: nowIso(),
    stdout: 'no_fuzz_targets_detected',
    stderr: '',
  });
}

report.checks.push(runStep('formal_invariants', 'npm', ['run', '-s', 'formal:invariants:run']));
report.checks.push(
  runStep('enterprise_hardening_strict', 'cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'core/layer0/ops/Cargo.toml',
    '--bin',
    'infring-ops',
    '--',
    'enterprise-hardening',
    'run',
    '--strict=1',
  ]),
);

report.ok = report.checks.every((row) => row.ok === true);
report.failed_checks = report.checks.filter((row) => row.ok !== true).map((row) => row.id);
report.finished_at = nowIso();

const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `nightly_fuzz_chaos_report_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'nightly_fuzz_chaos_report_latest.json');
fs.writeFileSync(stampedPath, `${JSON.stringify(report, null, 2)}\n`);
fs.writeFileSync(latestPath, `${JSON.stringify(report, null, 2)}\n`);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
