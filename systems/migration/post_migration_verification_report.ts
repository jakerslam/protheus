#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-MIGR-005
 * Post-migration verification + completion report.
 */

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.POST_MIGRATION_VERIFICATION_REPORT_POLICY_PATH
  ? path.resolve(process.env.POST_MIGRATION_VERIFICATION_REPORT_POLICY_PATH)
  : path.join(ROOT, 'config', 'post_migration_verification_report_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/migration/post_migration_verification_report.js run [--migration-id=<id>] [--apply=1|0] [--strict=1|0] [--telemetry-consent=1|0] [--policy=<path>]');
  console.log('  node systems/migration/post_migration_verification_report.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: false,
    signing: {
      key_env: 'PROTHEUS_MIGRATION_SIGNING_KEY',
      default_key: 'migration_dev_key',
      algorithm: 'sha256'
    },
    expected_surfaces: [
      { id: 'config', path: 'config', required: true },
      { id: 'habits', path: 'habits', required: false },
      { id: 'vault', path: 'secrets/vault', required: false },
      { id: 'memory', path: 'memory', required: true },
      { id: 'science', path: 'state/science', required: false },
      { id: 'routing', path: 'state/routing', required: false }
    ],
    core_bridge: {
      registry_path: 'state/migration/core_bridge/registry.json',
      checkpoints_root: 'state/migration/core_bridge/checkpoints'
    },
    paths: {
      latest_path: 'state/migration/post_migration_verification/latest.json',
      receipts_path: 'state/migration/post_migration_verification/receipts.jsonl',
      reports_root: 'state/migration/post_migration_verification/reports'
    }
  };
}

function normalizeSurface(raw: AnyObj) {
  if (!raw || typeof raw !== 'object') return null;
  const id = normalizeToken(raw.id || '', 80);
  const relPath = cleanText(raw.path || '', 280).replace(/^\/+/, '');
  if (!id || !relPath) return null;
  return {
    id,
    path: relPath,
    required: raw.required !== false
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const signing = raw.signing && typeof raw.signing === 'object' ? raw.signing : {};
  const bridge = raw.core_bridge && typeof raw.core_bridge === 'object' ? raw.core_bridge : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  const surfaces = Array.isArray(raw.expected_surfaces)
    ? raw.expected_surfaces.map((row: AnyObj) => normalizeSurface(row)).filter(Boolean)
    : base.expected_surfaces.map((row: AnyObj) => normalizeSurface(row)).filter(Boolean);

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    signing: {
      key_env: cleanText(signing.key_env || base.signing.key_env, 120),
      default_key: cleanText(signing.default_key || base.signing.default_key, 240),
      algorithm: cleanText(signing.algorithm || base.signing.algorithm, 40)
    },
    expected_surfaces: surfaces.length ? surfaces : base.expected_surfaces,
    core_bridge: {
      registry_path: resolvePath(bridge.registry_path, base.core_bridge.registry_path),
      checkpoints_root: resolvePath(bridge.checkpoints_root, base.core_bridge.checkpoints_root)
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      reports_root: resolvePath(paths.reports_root, base.paths.reports_root)
    },
    policy_path: path.resolve(policyPath)
  };
}

function sign(policy: AnyObj, payload: AnyObj) {
  const envName = cleanText(policy.signing.key_env || 'PROTHEUS_MIGRATION_SIGNING_KEY', 120) || 'PROTHEUS_MIGRATION_SIGNING_KEY';
  const secret = cleanText(process.env[envName] || policy.signing.default_key || 'migration_dev_key', 400) || 'migration_dev_key';
  return {
    algorithm: cleanText(policy.signing.algorithm || 'sha256', 40) || 'sha256',
    key_id: stableHash(`${envName}:${secret}`, 12),
    signature: stableHash(`${JSON.stringify(payload)}|${secret}`, 48)
  };
}

function writeReceipt(policy: AnyObj, payload: AnyObj) {
  const row = {
    ts: nowIso(),
    schema_id: 'post_migration_verification_report_receipt',
    schema_version: '1.0',
    ...payload
  };
  row.signature = sign(policy, {
    type: row.type,
    ok: row.ok === true,
    migration_id: row.migration_id || null,
    finalized: row.finalized === true
  });
  writeJsonAtomic(policy.paths.latest_path, row);
  appendJsonl(policy.paths.receipts_path, row);
  return row;
}

function existsPath(filePath: string) {
  try {
    return fs.existsSync(filePath);
  } catch {
    return false;
  }
}

function fileSize(filePath: string) {
  try {
    return Number(fs.statSync(filePath).size || 0);
  } catch {
    return 0;
  }
}

function runVerification(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, false);
  const telemetryConsent = toBool(args['telemetry-consent'] || args.telemetry_consent, false);
  const registry = readJson(policy.core_bridge.registry_path, {
    schema_version: '1.0',
    latest_migration_id: null,
    migrations: {}
  });

  const migrationId = cleanText(args['migration-id'] || args.migration_id || registry.latest_migration_id || '', 120);
  if (!migrationId) {
    return writeReceipt(policy, {
      ok: false,
      type: 'post_migration_verification_report_run',
      error: 'migration_id_required',
      strict,
      apply
    });
  }

  const checkpointPath = path.join(policy.core_bridge.checkpoints_root, migrationId, 'checkpoint.json');
  const checkpoint = readJson(checkpointPath, null);
  if (!checkpoint || typeof checkpoint !== 'object') {
    return writeReceipt(policy, {
      ok: false,
      type: 'post_migration_verification_report_run',
      error: 'checkpoint_not_found',
      migration_id: migrationId,
      checkpoint_path: rel(checkpointPath),
      strict,
      apply
    });
  }

  const sourceWorkspace = cleanText(checkpoint.source_workspace || '', 400);
  const targetWorkspace = cleanText(checkpoint.target_workspace || '', 400);
  const touched = Array.isArray(checkpoint.touched_files)
    ? checkpoint.touched_files.map((v: unknown) => cleanText(v, 400)).filter(Boolean)
    : [];

  const touchedMissing = touched.filter((relPath: string) => !existsPath(path.join(targetWorkspace, relPath)));
  const touchedBytes = touched.reduce((acc: number, relPath: string) => acc + fileSize(path.join(targetWorkspace, relPath)), 0);

  const surfaceChecks = policy.expected_surfaces.map((surface: AnyObj) => {
    const srcAbs = path.join(sourceWorkspace, surface.path);
    const dstAbs = path.join(targetWorkspace, surface.path);
    const srcExists = existsPath(srcAbs);
    const dstExists = existsPath(dstAbs);
    const pass = !srcExists || dstExists;
    return {
      id: surface.id,
      path: surface.path,
      required: !!surface.required,
      source_exists: srcExists,
      target_exists: dstExists,
      pass
    };
  });

  const requiredSurfaceFailures = surfaceChecks.filter((row: AnyObj) => row.required && row.pass !== true);
  const optionalSurfaceMisses = surfaceChecks.filter((row: AnyObj) => !row.required && row.pass !== true);

  const transferSummary = checkpoint.transfer_summary && typeof checkpoint.transfer_summary === 'object'
    ? checkpoint.transfer_summary
    : {};
  const plannedBytes = Number(transferSummary.bytes_total || 0);
  const plannedFiles = Number(transferSummary.file_count || 0);

  const checks = {
    checkpoint_exists: true,
    touched_files_present: touchedMissing.length === 0,
    required_surfaces_present: requiredSurfaceFailures.length === 0,
    transfer_file_count_nonzero: plannedFiles > 0,
    migration_not_rolled_back: !(
      registry.migrations
      && registry.migrations[migrationId]
      && registry.migrations[migrationId].rolled_back === true
    )
  };

  const pass = Object.values(checks).every(Boolean);
  const reportId = `pmvr_${stableHash(`${migrationId}|${pass}|${plannedFiles}|${plannedBytes}`, 12)}`;
  const reportPath = path.join(policy.paths.reports_root, `${reportId}.json`);

  const reportPayload = {
    schema_id: 'post_migration_completion_report',
    schema_version: '1.0',
    report_id: reportId,
    ts: nowIso(),
    migration_id: migrationId,
    source_workspace: sourceWorkspace,
    target_workspace: targetWorkspace,
    checks,
    surface_checks: surfaceChecks,
    touched_files_missing: touchedMissing,
    transfer_metrics: {
      planned_file_count: plannedFiles,
      planned_bytes: plannedBytes,
      observed_touched_bytes: touchedBytes,
      observed_coverage_ratio: plannedBytes > 0 ? Number((touchedBytes / plannedBytes).toFixed(6)) : null
    },
    optional_surface_misses: optionalSurfaceMisses,
    telemetry: telemetryConsent
      ? {
          consent_granted: true,
          exported_at: nowIso(),
          summary: {
            planned_file_count: plannedFiles,
            observed_touched_bytes: touchedBytes,
            missing_touched_files: touchedMissing.length
          }
        }
      : {
          consent_granted: false,
          exported_at: null
        },
    verdict: pass ? 'pass' : 'fail'
  };

  fs.mkdirSync(path.dirname(reportPath), { recursive: true });
  writeJsonAtomic(reportPath, reportPayload);

  let finalized = false;
  if (apply && pass && registry.migrations && registry.migrations[migrationId]) {
    registry.migrations[migrationId].status = 'finalized';
    registry.migrations[migrationId].finalized_at = nowIso();
    registry.migrations[migrationId].completion_report_path = rel(reportPath);
    registry.migrations[migrationId].telemetry_consent = telemetryConsent;
    writeJsonAtomic(policy.core_bridge.registry_path, registry);
    finalized = true;
  }

  const out = {
    ok: strict ? pass : true,
    type: 'post_migration_verification_report_run',
    lane_id: 'V4-MIGR-005',
    migration_id: migrationId,
    strict,
    apply,
    telemetry_consent: telemetryConsent,
    checks,
    pass,
    report_path: rel(reportPath),
    finalized,
    required_surface_failures: requiredSurfaceFailures,
    touched_files_missing: touchedMissing,
    transfer_metrics: reportPayload.transfer_metrics
  };

  return writeReceipt(policy, out);
}

function status(policy: AnyObj) {
  return {
    ok: true,
    type: 'post_migration_verification_report_status',
    lane_id: 'V4-MIGR-005',
    enabled: policy.enabled,
    latest: readJson(policy.paths.latest_path, null),
    latest_path: rel(policy.paths.latest_path),
    receipts_path: rel(policy.paths.receipts_path),
    reports_root: rel(policy.paths.reports_root)
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') {
    usage();
    return;
  }

  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) emit({ ok: false, error: 'post_migration_verification_report_disabled' }, 1);

  if (cmd === 'run') {
    const out = runVerification(args, policy);
    emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'status') emit(status(policy));

  usage();
  process.exit(1);
}

main();
