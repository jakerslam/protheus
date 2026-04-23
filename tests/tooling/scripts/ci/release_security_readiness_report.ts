#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const DEFAULT_OUT = path.join(ARTIFACT_DIR, 'release_security_readiness_latest.json');
const REQUIRED_FILES = [
  '.github/workflows/release-security-artifacts.yml',
  'docs/client/RELEASE_SECURITY_CHECKLIST.md',
  'docs/client/releases/v0.2.0_migration_guide.md',
];
const REQUIRED_BLOCKER_IDS = ['HMAN-030'];

type StepResult = {
  id: string;
  command: string;
  args: string[];
  status: number;
  ok: boolean;
  started_at: string;
  ended_at: string;
  stdout: string;
  stderr: string;
};

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function nowIso(): string {
  return new Date().toISOString();
}

function tsSlug(iso: string): string {
  return clean(iso, 120).replaceAll(':', '-').replaceAll('.', '-');
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function hasPlaceholder(value: string): boolean {
  const token = clean(value, 280).toLowerCase();
  return token.includes('${') || token === 'tbd' || token === 'todo' || token === 'pending' || token === 'unknown';
}

function isCanonicalRelativePath(value: string): boolean {
  const token = clean(value, 400);
  if (!token) return false;
  if (token.includes('\\')) return false;
  if (token.startsWith('/') || token.startsWith('./') || token.startsWith('../')) return false;
  if (token.includes('//')) return false;
  const segments = token.split('/');
  if (segments.some((segment) => !segment || segment === '.' || segment === '..')) return false;
  return true;
}

function isCanonicalStepId(value: string): boolean {
  return /^[a-z0-9][a-z0-9_:-]*$/.test(clean(value, 120));
}

function isIso(value: string): boolean {
  return Number.isFinite(Date.parse(clean(value, 80)));
}

function parseArgs(argv: string[]) {
  const outFlag = argv.find((row) => clean(row, 480).startsWith('--out=')) || '';
  const strictFlag = argv.find((row) => clean(row, 120).startsWith('--strict=')) || '';
  const outRaw = outFlag ? clean(outFlag.slice(6), 400) : '';
  const strictRaw = strictFlag ? clean(strictFlag.slice(9), 40) : '';
  return {
    out: outRaw ? path.resolve(ROOT, outRaw) : DEFAULT_OUT,
    strict: parseBool(strictRaw, true),
  };
}

function runStep(id: string, command: string, args: string[]): StepResult {
  const startedAt = nowIso();
  const out = spawnSync(command, args, {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  const endedAt = nowIso();
  return {
    id,
    command,
    args: [...args],
    status: Number.isFinite(out.status) ? Number(out.status) : 1,
    ok: Number(out.status ?? 1) === 0,
    started_at: startedAt,
    ended_at: endedAt,
    stdout: String(out.stdout || '').trim(),
    stderr: String(out.stderr || '').trim(),
  };
}

function checkFile(pathRel: string) {
  const rel = clean(pathRel, 300);
  const abs = path.join(ROOT, rel);
  return {
    path: rel,
    exists: fs.existsSync(abs),
    canonical_path: isCanonicalRelativePath(rel),
  };
}

function arraySetEqual(left: string[], right: string[]): boolean {
  const a = [...new Set(left)].sort();
  const b = [...new Set(right)].sort();
  return a.length === b.length && a.every((value, index) => value === b[index]);
}

function buildReport(args: { out: string; strict: boolean }) {
  const policyFailures: string[] = [];
  const invalid: string[] = [];

  const outRel = clean(path.relative(ROOT, args.out).replace(/\\/g, '/'), 400);
  if (!outRel || outRel.startsWith('../')) policyFailures.push(`output_path_outside_repo:${outRel || args.out}`);
  if (!outRel.endsWith('.json')) policyFailures.push(`output_path_non_json:${outRel || args.out}`);
  if (!clean(args.out, 800).startsWith(clean(ARTIFACT_DIR, 800))) {
    policyFailures.push(`output_path_outside_artifact_dir:${outRel || args.out}`);
  }

  if (REQUIRED_FILES.length === 0) policyFailures.push('required_files_empty');
  const duplicateRequiredFiles = duplicateValues(REQUIRED_FILES.map((row) => clean(row, 300)));
  if (duplicateRequiredFiles.length > 0) {
    policyFailures.push(`required_files_duplicate:${duplicateRequiredFiles.join(',')}`);
  }
  const nonCanonicalRequiredFiles = REQUIRED_FILES.filter((row) => !isCanonicalRelativePath(row));
  if (nonCanonicalRequiredFiles.length > 0) {
    policyFailures.push(`required_files_noncanonical:${nonCanonicalRequiredFiles.join(',')}`);
  }
  const requiredFileBaselines = [
    '.github/workflows/release-security-artifacts.yml',
    'docs/client/RELEASE_SECURITY_CHECKLIST.md',
    'docs/client/releases/v0.2.0_migration_guide.md',
  ];
  const missingRequiredBaselines = requiredFileBaselines.filter((row) => !REQUIRED_FILES.includes(row));
  if (missingRequiredBaselines.length > 0) {
    policyFailures.push(`required_files_baseline_missing:${missingRequiredBaselines.join(',')}`);
  }

  const blockers = [
    {
      id: 'HMAN-030',
      description:
        'Public tagged release publication to GitHub/npm requires maintainer-owned authority and token custody.',
    },
  ];
  if (blockers.length === 0) policyFailures.push('blockers_empty');
  const blockerIds = blockers.map((row) => clean(row.id, 120));
  const duplicateBlockerIds = duplicateValues(blockerIds);
  if (duplicateBlockerIds.length > 0) {
    policyFailures.push(`blocker_ids_duplicate:${duplicateBlockerIds.join(',')}`);
  }
  const missingRequiredBlockerIds = REQUIRED_BLOCKER_IDS.filter((row) => !blockerIds.includes(row));
  if (missingRequiredBlockerIds.length > 0) {
    policyFailures.push(`blocker_ids_required_missing:${missingRequiredBlockerIds.join(',')}`);
  }
  for (const blocker of blockers) {
    if (!/^HMAN-\d{3,}$/.test(clean(blocker.id, 120))) {
      policyFailures.push(`blocker_id_noncanonical:${clean(blocker.id, 120) || 'missing'}`);
    }
    if (!clean(blocker.description, 400) || hasPlaceholder(blocker.description)) {
      policyFailures.push(`blocker_description_invalid:${clean(blocker.id, 120) || 'unknown'}`);
    }
  }

  const stepPlans: Array<{ id: string; command: string; args: string[] }> = [
    {
      id: 'enterprise_hardening_strict',
      command: 'cargo',
      args: [
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
      ],
    },
    {
      id: 'formal_invariants',
      command: 'npm',
      args: ['run', '-s', 'formal:invariants:run'],
    },
    {
      id: 'supply_chain_provenance_status',
      command: 'cargo',
      args: [
        'run',
        '--quiet',
        '--manifest-path',
        'core/layer0/ops/Cargo.toml',
        '--bin',
        'protheus-ops',
        '--',
        'supply-chain-provenance-v2',
        'status',
      ],
    },
  ];

  if (stepPlans.length === 0) policyFailures.push('step_plans_empty');
  const stepIds = stepPlans.map((row) => clean(row.id, 120));
  const duplicateStepIds = duplicateValues(stepIds);
  if (duplicateStepIds.length > 0) {
    policyFailures.push(`step_ids_duplicate:${duplicateStepIds.join(',')}`);
  }
  for (const step of stepPlans) {
    if (!isCanonicalStepId(step.id)) policyFailures.push(`step_id_noncanonical:${step.id}`);
    if (!clean(step.command, 80)) policyFailures.push(`step_command_missing:${step.id}`);
    if (!Array.isArray(step.args) || step.args.length === 0) {
      policyFailures.push(`step_args_missing:${step.id}`);
    }
  }
  const cargoManifestSteps = ['enterprise_hardening_strict', 'supply_chain_provenance_status'];
  for (const stepId of cargoManifestSteps) {
    const step = stepPlans.find((row) => row.id === stepId);
    if (!step || !step.args.includes('core/layer0/ops/Cargo.toml')) {
      policyFailures.push(`step_manifest_contract_missing:${stepId}`);
    }
  }
  const formalStep = stepPlans.find((row) => row.id === 'formal_invariants');
  if (!formalStep || !formalStep.args.includes('formal:invariants:run')) {
    policyFailures.push('step_formal_invariants_contract_missing');
  }

  const report: any = {
    type: 'release_security_readiness_report',
    ts: nowIso(),
    cwd: ROOT,
    acceptance_target: 'V6-SEC-001',
    checks: [] as StepResult[],
    files: [] as Array<{ path: string; exists: boolean; canonical_path: boolean }>,
    blockers,
    policy_failures: policyFailures,
  };

  report.files = REQUIRED_FILES.map((row) => checkFile(row));
  const duplicateFilePaths = duplicateValues(report.files.map((row: any) => clean(row.path, 300)));
  if (duplicateFilePaths.length > 0) invalid.push(`report_file_paths_duplicate:${duplicateFilePaths.join(',')}`);
  const nonCanonicalFilePaths = report.files.filter((row: any) => row.canonical_path !== true).map((row: any) => row.path);
  if (nonCanonicalFilePaths.length > 0) invalid.push(`report_file_paths_noncanonical:${nonCanonicalFilePaths.join(',')}`);

  report.checks = stepPlans.map((row) => runStep(row.id, row.command, row.args));
  const duplicateCheckIds = duplicateValues(report.checks.map((row: StepResult) => clean(row.id, 120)));
  if (duplicateCheckIds.length > 0) invalid.push(`check_ids_duplicate:${duplicateCheckIds.join(',')}`);
  for (const check of report.checks) {
    if (!Number.isInteger(check.status)) invalid.push(`${check.id}:status_non_integer`);
    if (check.ok !== (check.status === 0)) invalid.push(`${check.id}:ok_status_drift`);
    if (!isIso(check.started_at) || !isIso(check.ended_at)) invalid.push(`${check.id}:timestamp_invalid`);
    if (isIso(check.started_at) && isIso(check.ended_at)) {
      const start = Date.parse(check.started_at);
      const end = Date.parse(check.ended_at);
      if (end < start) invalid.push(`${check.id}:timestamp_order_invalid`);
    }
    if (!clean(check.command, 80)) invalid.push(`${check.id}:command_missing`);
    if (!Array.isArray(check.args) || check.args.length === 0) invalid.push(`${check.id}:args_missing`);
  }

  const filesOk = report.files.every((row: any) => row.exists === true);
  const checksOk = report.checks.every((row: StepResult) => row.ok === true);
  report.readiness_ok = filesOk && checksOk;
  report.failed_checks = report.checks.filter((row: StepResult) => row.ok !== true).map((row: StepResult) => row.id);
  report.missing_files = report.files.filter((row: any) => row.exists !== true).map((row: any) => row.path);
  report.finished_at = nowIso();

  const derivedFailedChecks = report.checks.filter((row: StepResult) => row.ok !== true).map((row: StepResult) => row.id);
  if (!arraySetEqual(report.failed_checks, derivedFailedChecks)) invalid.push('failed_checks_set_drift');
  const derivedMissingFiles = report.files.filter((row: any) => row.exists !== true).map((row: any) => row.path);
  if (!arraySetEqual(report.missing_files, derivedMissingFiles)) invalid.push('missing_files_set_drift');
  const recomputedReadinessOk = filesOk && checksOk;
  if (report.readiness_ok !== recomputedReadinessOk) invalid.push('readiness_ok_drift');

  const oldestStamp = tsSlug(report.finished_at);
  const stampedPath = path.join(ARTIFACT_DIR, `release_security_readiness_${oldestStamp}.json`);
  const latestPath = path.join(ARTIFACT_DIR, 'release_security_readiness_latest.json');
  if (!clean(stampedPath, 800).endsWith('.json')) invalid.push('stamped_artifact_path_suffix_invalid');
  if (!clean(latestPath, 800).endsWith('.json')) invalid.push('latest_artifact_path_suffix_invalid');
  report.artifact_paths = {
    stamped: stampedPath,
    latest: latestPath,
    requested_out: args.out,
  };

  const totalIssues = policyFailures.length + invalid.length;
  report.invalid = invalid;
  report.summary = {
    policy_failure_count: policyFailures.length,
    invalid_count: invalid.length,
    total_issue_count: totalIssues,
    missing_file_count: report.missing_files.length,
    failed_check_count: report.failed_checks.length,
    pass: totalIssues === 0 && report.readiness_ok === true,
  };
  report.ok = report.summary.pass === true;

  return report;
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport(args);
  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });

  const stamp = tsSlug(report.finished_at);
  const stampedPath = path.join(ARTIFACT_DIR, `release_security_readiness_${stamp}.json`);
  const latestPath = path.join(ARTIFACT_DIR, 'release_security_readiness_latest.json');

  fs.writeFileSync(stampedPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  fs.writeFileSync(latestPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  if (path.resolve(args.out) !== path.resolve(latestPath)) {
    fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  }

  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return report.ok ? 0 : 1;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run, buildReport };
