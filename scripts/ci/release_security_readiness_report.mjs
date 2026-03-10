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

function checkFile(pathRel) {
  const abs = path.join(ROOT, pathRel);
  return {
    path: pathRel,
    exists: fs.existsSync(abs),
  };
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const report = {
  type: 'release_security_readiness_report',
  ts: nowIso(),
  cwd: ROOT,
  acceptance_target: 'V6-SEC-001',
  checks: [],
  files: [],
  blockers: [
    {
      id: 'HMAN-030',
      description:
        'Public tagged release publication to GitHub/npm requires maintainer-owned authority and token custody.',
    },
  ],
};

report.files.push(checkFile('.github/workflows/release-security-artifacts.yml'));
report.files.push(checkFile('docs/client/RELEASE_SECURITY_CHECKLIST.md'));
report.files.push(checkFile('docs/client/releases/v0.2.0_migration_guide.md'));

report.checks.push(
  runStep('enterprise_hardening_strict', 'cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'core/layer0/ops/Cargo.toml',
    '--bin',
    'protheus-ops',
    '--',
    'enterprise-hardening',
    'run',
    '--strict=1',
  ]),
);
report.checks.push(runStep('formal_invariants', 'npm', ['run', '-s', 'formal:invariants:run']));
report.checks.push(
  runStep('supply_chain_provenance_status', 'cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'core/layer0/ops/Cargo.toml',
    '--bin',
    'protheus-ops',
    '--',
    'supply-chain-provenance-v2',
    'status',
  ]),
);

const filesOk = report.files.every((row) => row.exists === true);
const checksOk = report.checks.every((row) => row.ok === true);
report.readiness_ok = filesOk && checksOk;
report.ok = report.readiness_ok;
report.failed_checks = report.checks.filter((row) => row.ok !== true).map((row) => row.id);
report.missing_files = report.files.filter((row) => row.exists !== true).map((row) => row.path);
report.finished_at = nowIso();

const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `release_security_readiness_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'release_security_readiness_latest.json');
fs.writeFileSync(stampedPath, `${JSON.stringify(report, null, 2)}\n`);
fs.writeFileSync(latestPath, `${JSON.stringify(report, null, 2)}\n`);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
