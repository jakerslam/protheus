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
  const stdout = String(out.stdout || '').trim();
  const stderr = String(out.stderr || '').trim();
  let parsed = null;
  if (stdout.startsWith('{') && stdout.endsWith('}')) {
    try {
      parsed = JSON.parse(stdout);
    } catch {
      parsed = null;
    }
  }
  return {
    id,
    command,
    args,
    status: Number.isFinite(out.status) ? out.status : 1,
    ok: (out.status ?? 1) === 0,
    started_at: startedAt,
    ended_at: endedAt,
    stdout,
    stderr,
    parsed,
  };
}

fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

const report = {
  type: 'formal_verification_expansion_report',
  ts: nowIso(),
  cwd: ROOT,
  acceptance_target: 'V6-SEC-005',
  objective:
    'Reproducible proof checks cover constitution invariants, receipt-chain integrity, and conduit command-surface formal gates.',
  checks: [],
};

report.checks.push(runStep('formal_invariants', 'npm', ['run', '-s', 'formal:invariants:run']));
report.checks.push(runStep('formal_proof_runtime_gate', 'npm', ['run', '-s', 'ops:formal-proof:run']));
report.checks.push(runStep('critical_protocol_formal_suite', 'npm', ['run', '-s', 'ops:formal-suite:run']));
report.checks.push(
  runStep('critical_path_formal_verifier', 'npm', ['run', '-s', 'test:critical:path:formal']),
);
report.checks.push(runStep('proof_pack_threshold_gate', 'npm', ['run', '-s', 'ops:proof-pack:gate']));

report.coverage = {
  constitution_invariants: report.checks
    .filter((row) => row.id === 'formal_invariants' || row.id === 'critical_path_formal_verifier')
    .every((row) => row.ok === true),
  receipt_chain_validation: report.checks
    .filter((row) => row.id === 'formal_proof_runtime_gate' || row.id === 'proof_pack_threshold_gate')
    .every((row) => row.ok === true),
  conduit_command_surface_validation: report.checks
    .filter((row) => row.id === 'critical_protocol_formal_suite')
    .every((row) => row.ok === true),
};

report.ok = report.checks.every((row) => row.ok === true);
report.failed_checks = report.checks.filter((row) => row.ok !== true).map((row) => row.id);
report.finished_at = nowIso();

const stamp = tsSlug(report.finished_at);
const stampedPath = path.join(ARTIFACT_DIR, `formal_verification_expansion_${stamp}.json`);
const latestPath = path.join(ARTIFACT_DIR, 'formal_verification_expansion_latest.json');
fs.writeFileSync(stampedPath, `${JSON.stringify(report, null, 2)}\n`);
fs.writeFileSync(latestPath, `${JSON.stringify(report, null, 2)}\n`);

process.stdout.write(`${JSON.stringify(report)}\n`);
process.exit(report.ok ? 0 : 1);
