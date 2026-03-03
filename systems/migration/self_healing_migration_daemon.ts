#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-MIGR-004
 * Self-healing migration detector + consent-gated upgrader.
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
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

const DEFAULT_POLICY_PATH = process.env.SELF_HEALING_MIGRATION_DAEMON_POLICY_PATH
  ? path.resolve(process.env.SELF_HEALING_MIGRATION_DAEMON_POLICY_PATH)
  : path.join(ROOT, 'config', 'self_healing_migration_daemon_policy.json');
const REPO_ROOT = path.resolve(__dirname, '..', '..');

function usage() {
  console.log('Usage:');
  console.log('  node systems/migration/self_healing_migration_daemon.js scan [--workspace=<path>] [--apply=1|0] [--to=<repo>] [--consent-token=<token>] [--policy=<path>]');
  console.log('  node systems/migration/self_healing_migration_daemon.js upgrade --to=<repo> --workspace-target=<path> --consent-token=<token> [--policy=<path>]');
  console.log('  node systems/migration/self_healing_migration_daemon.js status [--policy=<path>]');
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
    detector: {
      legacy_remote_patterns: ['openclaw', 'legacy', 'community'],
      suggest_if_remote_missing: true,
      require_consent_for_apply: true,
      consent_token_prefix: 'MIGR-CONSENT-'
    },
    integration: {
      self_audit_suggestions_path: 'state/self_audit/illusion_integrity_suggestions.jsonl'
    },
    paths: {
      latest_path: 'state/migration/self_healing/latest.json',
      receipts_path: 'state/migration/self_healing/receipts.jsonl',
      suggestions_path: 'state/migration/self_healing/suggestions.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const signing = raw.signing && typeof raw.signing === 'object' ? raw.signing : {};
  const detector = raw.detector && typeof raw.detector === 'object' ? raw.detector : {};
  const integration = raw.integration && typeof raw.integration === 'object' ? raw.integration : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    signing: {
      key_env: cleanText(signing.key_env || base.signing.key_env, 120),
      default_key: cleanText(signing.default_key || base.signing.default_key, 280),
      algorithm: cleanText(signing.algorithm || base.signing.algorithm, 40)
    },
    detector: {
      legacy_remote_patterns: Array.isArray(detector.legacy_remote_patterns)
        ? detector.legacy_remote_patterns.map((v: unknown) => cleanText(v, 80).toLowerCase()).filter(Boolean)
        : base.detector.legacy_remote_patterns,
      suggest_if_remote_missing: toBool(detector.suggest_if_remote_missing, base.detector.suggest_if_remote_missing),
      require_consent_for_apply: toBool(detector.require_consent_for_apply, base.detector.require_consent_for_apply),
      consent_token_prefix: cleanText(detector.consent_token_prefix || base.detector.consent_token_prefix, 80)
    },
    integration: {
      self_audit_suggestions_path: resolvePath(integration.self_audit_suggestions_path, base.integration.self_audit_suggestions_path)
    },
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      suggestions_path: resolvePath(paths.suggestions_path, base.paths.suggestions_path)
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
    schema_id: 'self_healing_migration_daemon_receipt',
    schema_version: '1.0',
    ...payload
  };
  row.signature = sign(policy, {
    type: row.type,
    ok: row.ok === true,
    detector_id: row.detector_id || null,
    migration_triggered: row.migration_triggered === true
  });
  writeJsonAtomic(policy.paths.latest_path, row);
  appendJsonl(policy.paths.receipts_path, row);
  return row;
}

function runGit(cwd: string, argv: string[]) {
  const res = spawnSync('git', ['-C', cwd, ...argv], {
    cwd: REPO_ROOT,
    encoding: 'utf8'
  });
  return {
    status: Number.isFinite(res.status) ? Number(res.status) : 1,
    stdout: String(res.stdout || '').trim(),
    stderr: String(res.stderr || '').trim()
  };
}

function detectLegacyWorkspace(policy: AnyObj, workspaceRoot: string) {
  const detectorId = `det_${stableHash(`${workspaceRoot}|${nowIso()}`, 10)}`;
  const gitPresent = fs.existsSync(path.join(workspaceRoot, '.git'));
  const remoteRun = gitPresent ? runGit(workspaceRoot, ['remote', 'get-url', 'origin']) : { status: 1, stdout: '', stderr: 'git_missing' };
  const remoteUrl = remoteRun.status === 0 ? cleanText(remoteRun.stdout, 320) : null;
  const branchRun = gitPresent ? runGit(workspaceRoot, ['rev-parse', '--abbrev-ref', 'HEAD']) : { status: 1, stdout: '', stderr: 'git_missing' };
  const branch = branchRun.status === 0 ? cleanText(branchRun.stdout, 80) : null;

  const patterns = policy.detector.legacy_remote_patterns || [];
  const matchedPattern = remoteUrl
    ? patterns.find((pattern: string) => remoteUrl.toLowerCase().includes(String(pattern).toLowerCase())) || null
    : null;

  const needsMigration = !!matchedPattern || (!remoteUrl && policy.detector.suggest_if_remote_missing === true);
  const suggestion = needsMigration
    ? {
        ts: nowIso(),
        detector_id: detectorId,
        workspace: workspaceRoot,
        reason: matchedPattern ? 'legacy_remote_detected' : 'remote_missing',
        legacy_remote_pattern: matchedPattern,
        legacy_remote_url: remoteUrl,
        suggested_command: `protheusctl migrate --to=<org/repo> --workspace=${workspaceRoot}`,
        suggestion_class: 'migration_upgrade'
      }
    : null;

  return {
    detector_id: detectorId,
    workspace: workspaceRoot,
    git_present: gitPresent,
    remote_url: remoteUrl,
    branch,
    matched_pattern: matchedPattern,
    needs_migration: needsMigration,
    suggestion
  };
}

function appendSuggestion(policy: AnyObj, suggestion: AnyObj) {
  if (!suggestion) return;
  appendJsonl(policy.paths.suggestions_path, suggestion);
  appendJsonl(policy.integration.self_audit_suggestions_path, {
    ts: suggestion.ts || nowIso(),
    source: 'self_healing_migration_daemon',
    suggestion_class: suggestion.suggestion_class || 'migration_upgrade',
    summary: suggestion.reason,
    suggestion
  });
}

function consentValid(policy: AnyObj, tokenRaw: string) {
  const token = cleanText(tokenRaw || '', 200);
  if (!token) return false;
  if (policy.detector.require_consent_for_apply !== true) return true;
  const prefix = cleanText(policy.detector.consent_token_prefix || '', 80);
  return prefix ? token.startsWith(prefix) : token.length >= 8;
}

function triggerMigration(args: AnyObj, workspaceRoot: string, targetRepo: string, targetWorkspace: string, consentToken: string, migrationPolicyPath: string | null) {
  const script = path.join(REPO_ROOT, 'systems', 'migration', 'core_migration_bridge.js');
  if (!fs.existsSync(script)) {
    return {
      ok: false,
      error: 'core_migration_bridge_missing'
    };
  }

  const laneArgs = [
    script,
    'run',
    `--to=${targetRepo}`,
    `--workspace=${targetWorkspace}`,
    '--apply=1',
    `--consent-token=${consentToken}`
  ];
  if (migrationPolicyPath) laneArgs.push(`--policy=${migrationPolicyPath}`);

  const run = spawnSync('node', laneArgs, {
    cwd: REPO_ROOT,
    encoding: 'utf8',
    env: {
      ...process.env,
      OPENCLAW_WORKSPACE: workspaceRoot
    }
  });

  let payload = null;
  try { payload = JSON.parse(String(run.stdout || '').trim()); } catch {}
  return {
    ok: Number(run.status || 0) === 0 && payload && payload.ok === true,
    status: Number.isFinite(run.status) ? Number(run.status) : 1,
    payload,
    stderr: String(run.stderr || '').trim()
  };
}

function scan(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, false);
  const workspaceRootRaw = cleanText(args.workspace || ROOT, 400);
  const workspaceRoot = path.isAbsolute(workspaceRootRaw) ? workspaceRootRaw : path.resolve(ROOT, workspaceRootRaw);
  const targetRepo = cleanText(args.to || '', 300);
  const targetWorkspaceRaw = cleanText(args['workspace-target'] || args.workspace_target || '', 400);
  const targetWorkspace = targetWorkspaceRaw
    ? (path.isAbsolute(targetWorkspaceRaw) ? targetWorkspaceRaw : path.resolve(ROOT, targetWorkspaceRaw))
    : null;
  const consentToken = cleanText(args['consent-token'] || args.consent_token || '', 240);

  const detection = detectLegacyWorkspace(policy, workspaceRoot);
  if (detection.suggestion) appendSuggestion(policy, detection.suggestion);

  const out: AnyObj = {
    ok: true,
    type: 'self_healing_migration_daemon_scan',
    lane_id: 'V4-MIGR-004',
    strict,
    apply,
    detector_id: detection.detector_id,
    workspace: detection.workspace,
    git_present: detection.git_present,
    remote_url: detection.remote_url,
    branch: detection.branch,
    needs_migration: detection.needs_migration,
    matched_pattern: detection.matched_pattern,
    suggestion: detection.suggestion,
    consent_valid: consentValid(policy, consentToken),
    migration_triggered: false,
    migration_result: null
  };

  if (apply && detection.needs_migration) {
    if (!targetRepo || !targetWorkspace) {
      out.ok = false;
      out.error = 'target_repo_and_workspace_target_required';
      return writeReceipt(policy, out);
    }
    if (!consentValid(policy, consentToken)) {
      out.ok = false;
      out.error = 'valid_consent_token_required';
      return writeReceipt(policy, out);
    }

    const migrationPolicyPath = cleanText(args['migration-policy'] || args.migration_policy || '', 400) || null;
    const migration = triggerMigration(args, workspaceRoot, targetRepo, targetWorkspace, consentToken, migrationPolicyPath);
    out.migration_triggered = true;
    out.migration_result = {
      ok: migration.ok,
      status: migration.status,
      payload: migration.payload,
      stderr: migration.stderr
    };
    out.ok = migration.ok;
  }

  if (strict && detection.needs_migration && !apply) {
    out.ok = false;
    out.error = 'migration_needed_not_applied';
  }

  return writeReceipt(policy, out);
}

function status(policy: AnyObj) {
  return {
    ok: true,
    type: 'self_healing_migration_daemon_status',
    lane_id: 'V4-MIGR-004',
    enabled: policy.enabled,
    latest: readJson(policy.paths.latest_path, null),
    latest_path: rel(policy.paths.latest_path),
    receipts_path: rel(policy.paths.receipts_path),
    suggestions_path: rel(policy.paths.suggestions_path),
    self_audit_suggestions_path: rel(policy.integration.self_audit_suggestions_path)
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
  if (!policy.enabled) emit({ ok: false, error: 'self_healing_migration_daemon_disabled' }, 1);

  if (cmd === 'scan' || cmd === 'upgrade') {
    const normalizedArgs = {
      ...args,
      apply: cmd === 'upgrade' ? 1 : args.apply
    };
    const out = scan(normalizedArgs, policy);
    emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'status') emit(status(policy));

  usage();
  process.exit(1);
}

main();
