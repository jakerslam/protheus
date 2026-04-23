#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/legacy_process_runner_release_guard_current.json');
const CLOSURE_POLICY_PATH = path.join(
  ROOT,
  'client/runtime/config/production_readiness_closure_policy.json',
);

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]) {
  const parsed = {
    strict: false,
    out: DEFAULT_OUT,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) parsed.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) parsed.out = path.resolve(ROOT, clean(token.slice(6), 400));
  }
  return parsed;
}

function readSource(relPath: string): string {
  const abs = path.join(ROOT, relPath);
  return fs.existsSync(abs) ? fs.readFileSync(abs, 'utf8') : '';
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function buildReport() {
  const runnerPath = 'adapters/runtime/run_infring_ops.ts';
  const bridgePath = 'adapters/runtime/ops_lane_bridge.ts';
  const legacyHelperPath = 'adapters/runtime/dev_only/legacy_process_runner.ts';
  const processFallbackHelperPath = 'adapters/runtime/dev_only/ops_lane_process_fallback.ts';
  const runtimeManifestPath = 'client/runtime/config/install_runtime_manifest_v1.txt';
  const closurePolicy = readJson<any>(CLOSURE_POLICY_PATH, {});
  const deletionTargetDate = clean(
    closurePolicy?.legacy_process_runner_transition?.deletion_target_date,
    40,
  );
  const deletionTargetRelease = clean(
    closurePolicy?.legacy_process_runner_transition?.deletion_target_release,
    80,
  );
  const todayRaw = clean(process.env.INFRING_LEGACY_RUNNER_GUARD_TODAY, 40);
  const today = todayRaw || new Date().toISOString().slice(0, 10);
  const deletionDeadlineReached =
    Boolean(deletionTargetDate) && Number.isFinite(Date.parse(`${deletionTargetDate}T00:00:00Z`))
      ? Date.parse(`${today}T00:00:00Z`) >= Date.parse(`${deletionTargetDate}T00:00:00Z`)
      : false;
  const cutoffOverride = parseBool(process.env.INFRING_LEGACY_RUNNER_DELETION_BLOCKER, false);
  const blockerEvidencePath = clean(
    process.env.INFRING_LEGACY_RUNNER_BLOCKER_PATH || 'core/local/artifacts/release_blocker_rubric_current.json',
    400,
  );

  const runnerSource = readSource(runnerPath);
  const bridgeSource = readSource(bridgePath);
  const legacyHelperSource = readSource(legacyHelperPath);
  const processFallbackHelperSource = readSource(processFallbackHelperPath);
  const runtimeManifest = readSource(runtimeManifestPath);
  const blockerEvidence = readSource(blockerEvidencePath);
  const legacyHelperExists = fs.existsSync(path.join(ROOT, legacyHelperPath));
  const processFallbackHelperExists = fs.existsSync(path.join(ROOT, processFallbackHelperPath));
  const blockerEvidenceExists = fs.existsSync(path.join(ROOT, blockerEvidencePath));
  const blockerEvidenceReferencesLegacyRunner =
    blockerEvidence.includes('legacy_process_runner') || blockerEvidence.includes('ops_lane_process_fallback');

  const checks = [
    {
      id: 'runner_entrypoint_has_no_spawn_sync',
      ok: !runnerSource.includes('spawnSync('),
      detail: 'run_infring_ops.ts must stay resident-first',
    },
    {
      id: 'bridge_entrypoint_has_no_spawn_sync',
      ok: !bridgeSource.includes('spawnSync('),
      detail: 'ops_lane_bridge.ts must not embed process fallback execution',
    },
    {
      id: 'runner_entrypoint_uses_dev_only_helper',
      ok: runnerSource.includes("./dev_only/legacy_process_runner.ts"),
      detail: 'legacy runner must be loaded from adapters/runtime/dev_only',
    },
    {
      id: 'bridge_entrypoint_uses_dev_only_helper',
      ok: bridgeSource.includes("./dev_only/ops_lane_process_fallback.ts"),
      detail: 'process fallback helper must be loaded from adapters/runtime/dev_only',
    },
    {
      id: 'legacy_helper_marked_dev_only',
      ok:
        legacyHelperSource.includes('legacy_process_runner_dev_only') &&
        legacyHelperSource.includes('spawnSync('),
      detail: 'legacy helper must be explicitly marked dev-only',
    },
    {
      id: 'process_fallback_helper_marked_dev_only',
      ok:
        processFallbackHelperSource.includes('process_fallback_dev_only') &&
        processFallbackHelperSource.includes('spawnSync('),
      detail: 'process fallback helper must be explicitly marked dev-only',
    },
    {
      id: 'runtime_manifest_excludes_dev_only_helpers',
      ok:
        !runtimeManifest.includes('adapters/runtime/dev_only/') &&
        !runtimeManifest.includes('legacy_process_runner.ts') &&
        !runtimeManifest.includes('ops_lane_process_fallback.ts'),
      detail: 'install runtime manifest must not ship dev-only legacy helpers',
    },
    {
      id: 'legacy_runner_deletion_target_declared',
      ok: Boolean(deletionTargetDate) && Boolean(deletionTargetRelease),
      detail: `target_date=${deletionTargetDate || 'missing'};target_release=${deletionTargetRelease || 'missing'}`,
    },
    {
      id: 'legacy_runner_deleted_by_cutoff',
      ok: !deletionDeadlineReached || cutoffOverride || !legacyHelperExists,
      detail: `today=${today};target_date=${deletionTargetDate || 'missing'};exists=${String(legacyHelperExists)};override=${String(cutoffOverride)}`,
    },
    {
      id: 'process_fallback_deleted_by_cutoff',
      ok: !deletionDeadlineReached || cutoffOverride || !processFallbackHelperExists,
      detail: `today=${today};target_date=${deletionTargetDate || 'missing'};exists=${String(processFallbackHelperExists)};override=${String(cutoffOverride)}`,
    },
    {
      id: 'legacy_runner_override_requires_blocker_evidence',
      ok:
        !deletionDeadlineReached ||
        !cutoffOverride ||
        ((!legacyHelperExists && !processFallbackHelperExists) ||
          (blockerEvidenceExists && blockerEvidenceReferencesLegacyRunner)),
      detail: `override=${String(cutoffOverride)};deadline_reached=${String(deletionDeadlineReached)};blocker_path=${blockerEvidencePath};blocker_exists=${String(blockerEvidenceExists)};legacy_ref=${String(blockerEvidenceReferencesLegacyRunner)}`,
    },
  ];

  return {
    ok: checks.every((row) => row.ok),
    type: 'legacy_process_runner_release_guard',
    generated_at: new Date().toISOString(),
    deletion_target_date: deletionTargetDate || null,
    deletion_target_release: deletionTargetRelease || null,
    deletion_deadline_reached: deletionDeadlineReached,
    cutoff_override_active: cutoffOverride,
    blocker_evidence_path: blockerEvidencePath,
    blocker_evidence_exists: blockerEvidenceExists,
    checks,
    failed: checks.filter((row) => !row.ok).map((row) => row.id),
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport();
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { buildReport, run };
