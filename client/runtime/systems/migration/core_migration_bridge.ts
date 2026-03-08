#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');

const SCHEMA_ID = 'core_migration_bridge_receipt';
const SCHEMA_VERSION = '1.0';

type JsonMap = Record<string, any>;

type TransferSurface = {
  id: string;
  source: string;
  target: string;
  required?: boolean;
};

type Policy = {
  enabled?: boolean;
  strict_default?: boolean;
  git?: {
    update_remote?: boolean;
    remote_name?: string;
    auto_init_target_repo?: boolean;
  };
  workspace?: {
    default_parent?: string;
  };
  transfer_surfaces?: TransferSurface[];
  paths?: {
    latest_path?: string;
    receipts_path?: string;
    checkpoints_root?: string;
    registry_path?: string;
  };
};

type ParsedArgs = {
  command: string;
  positional: string[];
  flags: JsonMap;
};

type RegistryEntry = {
  migration_id: string;
  ts: string;
  source_root: string;
  target_workspace: string;
  to_repo: string | null;
  surfaces: Array<{
    id: string;
    source: string;
    target: string;
    source_exists: boolean;
    target_existed_before: boolean;
    checkpoint_before: string | null;
    applied: boolean;
  }>;
  git: {
    enabled: boolean;
    remote_name: string;
    action: string;
    previous_url: string | null;
    target_url: string | null;
  };
  checkpoint_root: string;
};

type Registry = {
  schema_id: string;
  schema_version: string;
  updated_at: string;
  migrations: RegistryEntry[];
};

function nowIso() {
  return new Date().toISOString();
}

function repoRoot() {
  if (process.env.OPENCLAW_WORKSPACE && String(process.env.OPENCLAW_WORKSPACE).trim()) {
    return path.resolve(String(process.env.OPENCLAW_WORKSPACE).trim());
  }
  return path.resolve(__dirname, '..', '..');
}

function parseArgs(argv: string[]): ParsedArgs {
  const positional: string[] = [];
  const flags: JsonMap = {};
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      positional.push(token);
      continue;
    }
    const eq = token.indexOf('=');
    if (eq >= 0) {
      flags[token.slice(2, eq)] = token.slice(eq + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      flags[key] = String(next);
      i += 1;
      continue;
    }
    flags[key] = true;
  }
  const command = String(positional[0] || 'status').trim().toLowerCase() || 'status';
  return {
    command,
    positional: positional.slice(1),
    flags
  };
}

function usage() {
  process.stdout.write('Usage:\n');
  process.stdout.write('  node client/runtime/systems/migration/core_migration_bridge.js run --to=<owner/repo|url> [--workspace=<path>] [--apply=1|0] [--policy=<path>]\n');
  process.stdout.write('  node client/runtime/systems/migration/core_migration_bridge.js rollback --migration-id=<id> [--apply=1|0] [--approval-note=<note>] [--policy=<path>]\n');
  process.stdout.write('  node client/runtime/systems/migration/core_migration_bridge.js status [--policy=<path>]\n');
}

function toBool(value: any, fallback = false) {
  const raw = String(value == null ? '' : value).trim().toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function asString(value: any, fallback = '') {
  const raw = String(value == null ? '' : value).trim();
  return raw || fallback;
}

function resolvePath(root: string, value: any, fallback: string) {
  const raw = asString(value, fallback);
  if (path.isAbsolute(raw)) return raw;
  return path.resolve(root, raw);
}

function readJson(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function ensureDirFor(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function writeJson(filePath: string, value: any) {
  ensureDirFor(filePath);
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function appendJsonl(filePath: string, value: any) {
  ensureDirFor(filePath);
  fs.appendFileSync(filePath, `${JSON.stringify(value)}\n`, 'utf8');
}

function canonicalize(value: any): any {
  if (Array.isArray(value)) return value.map((row) => canonicalize(row));
  if (value && typeof value === 'object') {
    const out: JsonMap = {};
    for (const key of Object.keys(value).sort()) {
      out[key] = canonicalize(value[key]);
    }
    return out;
  }
  return value;
}

function hashReceipt(value: any) {
  const canonical = JSON.stringify(canonicalize(value));
  return crypto.createHash('sha256').update(canonical).digest('hex');
}

function loadPolicy(root: string, flags: JsonMap): { policyPath: string, policy: Policy } {
  const policyPath = resolvePath(
    root,
    flags.policy,
    'client/runtime/config/core_migration_bridge_policy.json'
  );
  const policy = readJson(policyPath, {}) as Policy;
  return {
    policyPath,
    policy
  };
}

function resolveWorkspaceTarget(sourceRoot: string, policy: Policy, flags: JsonMap): string {
  if (asString(flags.workspace)) {
    return resolvePath(sourceRoot, flags.workspace, '.');
  }
  const defaultParent = asString(policy && policy.workspace && policy.workspace.default_parent, '..');
  return resolvePath(sourceRoot, defaultParent, '.');
}

function copyPath(src: string, dst: string) {
  ensureDirFor(path.join(dst, '__seed__'));
  fs.rmSync(path.join(dst, '__seed__'), { force: true });
  fs.cpSync(src, dst, { recursive: true, force: true });
}

function removePath(target: string) {
  if (!fs.existsSync(target)) return;
  fs.rmSync(target, { recursive: true, force: true });
}

function runGit(cwd: string, args: string[]) {
  const out = spawnSync('git', ['-C', cwd, ...args], { encoding: 'utf8' });
  return {
    ok: out.status === 0,
    status: Number.isFinite(out.status) ? Number(out.status) : 1,
    stdout: String(out.stdout || ''),
    stderr: String(out.stderr || '')
  };
}

function normalizeRepoUrl(raw: string) {
  const token = asString(raw);
  if (!token) return '';
  if (/^https?:\/\//i.test(token) || /^git@/i.test(token)) return token;
  return `https://github.com/${token.replace(/^\/+|\/+$/g, '')}.git`;
}

function migrationId() {
  return `mig_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
}

function loadRegistry(registryPath: string): Registry {
  const raw = readJson(registryPath, null);
  if (!raw || typeof raw !== 'object') {
    return {
      schema_id: 'core_migration_bridge_registry',
      schema_version: '1.0',
      updated_at: nowIso(),
      migrations: []
    };
  }
  const list = Array.isArray(raw.migrations) ? raw.migrations : [];
  return {
    schema_id: 'core_migration_bridge_registry',
    schema_version: '1.0',
    updated_at: asString(raw.updated_at, nowIso()),
    migrations: list
  };
}

function saveRegistry(registryPath: string, registry: Registry) {
  registry.updated_at = nowIso();
  writeJson(registryPath, registry);
}

function buildPaths(root: string, policy: Policy) {
  return {
    latestPath: resolvePath(root, policy && policy.paths && policy.paths.latest_path, 'state/migration/core_bridge/latest.json'),
    receiptsPath: resolvePath(root, policy && policy.paths && policy.paths.receipts_path, 'state/migration/core_bridge/receipts.jsonl'),
    checkpointsRoot: resolvePath(root, policy && policy.paths && policy.paths.checkpoints_root, 'state/migration/core_bridge/checkpoints'),
    registryPath: resolvePath(root, policy && policy.paths && policy.paths.registry_path, 'state/migration/core_bridge/registry.json')
  };
}

function receiptBase(type: string, extra: JsonMap = {}) {
  const payload: JsonMap = {
    ok: true,
    type,
    ts: nowIso(),
    schema_id: SCHEMA_ID,
    schema_version: SCHEMA_VERSION,
    ...extra
  };
  payload.receipt_hash = hashReceipt(payload);
  return payload;
}

function fail(type: string, message: string, extra: JsonMap = {}) {
  const payload: JsonMap = {
    ok: false,
    type,
    ts: nowIso(),
    schema_id: SCHEMA_ID,
    schema_version: SCHEMA_VERSION,
    error: message,
    ...extra
  };
  payload.receipt_hash = hashReceipt(payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(1);
}

function commandRun(root: string, parsed: ParsedArgs, policyPath: string, policy: Policy) {
  if (policy && policy.enabled === false) {
    fail('core_migration_bridge_error', 'migration_disabled_by_policy', { policy_path: policyPath });
  }
  const apply = toBool(parsed.flags.apply, false);
  const strict = toBool(parsed.flags.strict, !!(policy && policy.strict_default));
  const targetWorkspace = resolveWorkspaceTarget(root, policy, parsed.flags);
  const toRepo = asString(parsed.flags.to, '');
  const surfaces = Array.isArray(policy && policy.transfer_surfaces)
    ? policy.transfer_surfaces
    : [];
  const gitPolicy = {
    updateRemote: toBool(policy && policy.git && policy.git.update_remote, true),
    remoteName: asString(policy && policy.git && policy.git.remote_name, 'origin'),
    autoInit: toBool(policy && policy.git && policy.git.auto_init_target_repo, true)
  };
  const paths = buildPaths(root, policy);
  const id = migrationId();
  const checkpointRoot = path.join(paths.checkpointsRoot, id);
  const checkpointSurfaceRoot = path.join(checkpointRoot, 'surfaces');

  const surfacePlan = surfaces.map((surface: TransferSurface) => {
    const src = path.resolve(root, surface.source);
    const dst = path.resolve(targetWorkspace, surface.target);
    const srcExists = fs.existsSync(src);
    const dstExists = fs.existsSync(dst);
    return {
      id: asString(surface.id, stableSurfaceId(surface.source, surface.target)),
      source: src,
      target: dst,
      required: surface.required === true,
      source_exists: srcExists,
      target_existed_before: dstExists
    };
  });

  const missingRequired = surfacePlan
    .filter((row) => row.required && !row.source_exists)
    .map((row) => row.id);

  if (strict && missingRequired.length > 0) {
    fail('core_migration_bridge_error', 'required_surface_missing', {
      policy_path: policyPath,
      missing_required_surfaces: missingRequired
    });
  }

  let remoteAction = 'skipped';
  let previousRemoteUrl: string | null = null;
  let targetRemoteUrl: string | null = null;
  if (gitPolicy.updateRemote && toRepo) {
    targetRemoteUrl = normalizeRepoUrl(toRepo);
  }

  if (apply) {
    fs.mkdirSync(targetWorkspace, { recursive: true });
    if (gitPolicy.updateRemote && toRepo) {
      const gitDir = path.join(targetWorkspace, '.git');
      if (!fs.existsSync(gitDir) && gitPolicy.autoInit) {
        const init = runGit(targetWorkspace, ['init']);
        if (!init.ok) {
          fail('core_migration_bridge_error', 'git_init_failed', {
            stderr: init.stderr,
            stdout: init.stdout
          });
        }
      }
      const probe = runGit(targetWorkspace, ['remote', 'get-url', gitPolicy.remoteName]);
      if (probe.ok) {
        previousRemoteUrl = probe.stdout.trim() || null;
        if (previousRemoteUrl === targetRemoteUrl) {
          remoteAction = 'unchanged';
        } else {
          const set = runGit(targetWorkspace, ['remote', 'set-url', gitPolicy.remoteName, targetRemoteUrl as string]);
          if (!set.ok) {
            fail('core_migration_bridge_error', 'git_remote_set_url_failed', {
              stderr: set.stderr,
              stdout: set.stdout
            });
          }
          remoteAction = 'set-url';
        }
      } else {
        const add = runGit(targetWorkspace, ['remote', 'add', gitPolicy.remoteName, targetRemoteUrl as string]);
        if (!add.ok) {
          fail('core_migration_bridge_error', 'git_remote_add_failed', {
            stderr: add.stderr,
            stdout: add.stdout
          });
        }
        remoteAction = 'add';
      }
    }

    fs.mkdirSync(checkpointSurfaceRoot, { recursive: true });
    for (const row of surfacePlan) {
      if (!row.source_exists) continue;
      const cpBefore = path.join(checkpointSurfaceRoot, row.id, 'before');
      if (row.target_existed_before) {
        fs.mkdirSync(path.dirname(cpBefore), { recursive: true });
        copyPath(row.target, cpBefore);
      }
      removePath(row.target);
      copyPath(row.source, row.target);
      (row as any).checkpoint_before = row.target_existed_before ? cpBefore : null;
      (row as any).applied = true;
    }
  }

  const appliedSurfaces = surfacePlan.map((row: any) => ({
    id: row.id,
    source: relPath(root, row.source),
    target: relPath(root, row.target),
    source_exists: row.source_exists,
    target_existed_before: row.target_existed_before,
    checkpoint_before: row.checkpoint_before ? relPath(root, row.checkpoint_before) : null,
    applied: apply && row.source_exists
  }));

  if (apply) {
    const registry = loadRegistry(paths.registryPath);
    const entry: RegistryEntry = {
      migration_id: id,
      ts: nowIso(),
      source_root: root,
      target_workspace: targetWorkspace,
      to_repo: toRepo || null,
      surfaces: appliedSurfaces,
      git: {
        enabled: gitPolicy.updateRemote,
        remote_name: gitPolicy.remoteName,
        action: remoteAction,
        previous_url: previousRemoteUrl,
        target_url: targetRemoteUrl
      },
      checkpoint_root: checkpointRoot
    };
    registry.migrations.push(entry);
    saveRegistry(paths.registryPath, registry);
  }

  const payload = receiptBase('core_migration_bridge_run', {
    policy_path: relPath(root, policyPath),
    migration_id: id,
    source_root: root,
    target_workspace: targetWorkspace,
    to_repo: toRepo || null,
    strict,
    applied: apply,
    result: apply ? 'applied' : 'planned',
    remote_action: remoteAction,
    surfaces: appliedSurfaces,
    missing_required_surfaces: missingRequired
  });
  writeJson(paths.latestPath, payload);
  appendJsonl(paths.receiptsPath, payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(0);
}

function stableSurfaceId(sourceRel: string, targetRel: string) {
  return crypto
    .createHash('sha256')
    .update(`${sourceRel}::${targetRel}`)
    .digest('hex')
    .slice(0, 12);
}

function commandRollback(root: string, parsed: ParsedArgs, policyPath: string, policy: Policy) {
  const apply = toBool(parsed.flags.apply, false);
  const migrationIdFlag = asString(
    parsed.flags['migration-id'] || parsed.flags['migration_id'] || parsed.flags.id,
    ''
  );
  const paths = buildPaths(root, policy);
  const registry = loadRegistry(paths.registryPath);
  if (registry.migrations.length === 0) {
    fail('core_migration_bridge_error', 'rollback_registry_empty', { policy_path: policyPath });
  }
  const entry = migrationIdFlag
    ? registry.migrations.find((row) => row.migration_id === migrationIdFlag)
    : registry.migrations[registry.migrations.length - 1];
  if (!entry) {
    fail('core_migration_bridge_error', 'rollback_migration_missing', {
      migration_id: migrationIdFlag
    });
  }

  let remoteAction = 'skipped';
  if (apply) {
    for (const surface of entry.surfaces || []) {
      const targetAbs = path.resolve(entry.target_workspace, surface.target);
      const checkpointBefore = surface.checkpoint_before
        ? path.resolve(root, surface.checkpoint_before)
        : null;
      if (surface.target_existed_before && checkpointBefore && fs.existsSync(checkpointBefore)) {
        removePath(targetAbs);
        copyPath(checkpointBefore, targetAbs);
      } else if (!surface.target_existed_before) {
        removePath(targetAbs);
      }
    }

    if (entry.git && entry.git.enabled) {
      const remoteName = asString(entry.git.remote_name, 'origin');
      if (entry.git.previous_url) {
        const set = runGit(entry.target_workspace, ['remote', 'set-url', remoteName, entry.git.previous_url]);
        if (!set.ok) {
          fail('core_migration_bridge_error', 'rollback_remote_restore_failed', {
            stderr: set.stderr,
            stdout: set.stdout
          });
        }
        remoteAction = 'set-url';
      } else if (entry.git.action === 'add') {
        const rm = runGit(entry.target_workspace, ['remote', 'remove', remoteName]);
        if (!rm.ok) {
          fail('core_migration_bridge_error', 'rollback_remote_remove_failed', {
            stderr: rm.stderr,
            stdout: rm.stdout
          });
        }
        remoteAction = 'remove';
      }
    }
  }

  const payload = receiptBase('core_migration_bridge_rollback', {
    policy_path: relPath(root, policyPath),
    migration_id: entry.migration_id,
    approval_note: asString(parsed.flags['approval-note'] || parsed.flags.approval_note, ''),
    applied: apply,
    result: apply ? 'applied' : 'planned',
    remote_action: remoteAction
  });
  writeJson(paths.latestPath, payload);
  appendJsonl(paths.receiptsPath, payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(0);
}

function commandStatus(root: string, policyPath: string, policy: Policy) {
  const paths = buildPaths(root, policy);
  const latest = readJson(paths.latestPath, null);
  const registry = loadRegistry(paths.registryPath);
  const payload = receiptBase('core_migration_bridge_status', {
    policy_path: relPath(root, policyPath),
    enabled: policy && policy.enabled !== false,
    latest: latest,
    registry_count: registry.migrations.length,
    latest_migration_id: registry.migrations.length
      ? registry.migrations[registry.migrations.length - 1].migration_id
      : null
  });
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(0);
}

function relPath(root: string, inputPath: string) {
  const abs = path.resolve(inputPath);
  const rel = path.relative(root, abs);
  if (!rel || rel.startsWith('..')) return abs;
  return rel;
}

function main() {
  const root = repoRoot();
  const parsed = parseArgs(process.argv.slice(2));
  if (parsed.command === 'help' || parsed.command === '--help' || parsed.command === '-h') {
    usage();
    process.exit(0);
  }
  const { policyPath, policy } = loadPolicy(root, parsed.flags);
  if (parsed.command === 'run') {
    commandRun(root, parsed, policyPath, policy);
    return;
  }
  if (parsed.command === 'rollback') {
    commandRollback(root, parsed, policyPath, policy);
    return;
  }
  if (parsed.command === 'status') {
    commandStatus(root, policyPath, policy);
    return;
  }
  fail('core_migration_bridge_error', 'unknown_command', { command: parsed.command });
}

if (require.main === module) {
  main();
}
