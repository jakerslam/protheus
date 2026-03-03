#!/usr/bin/env node
'use strict';
export {};

/**
 * V4-MIGR-001
 * Core migration bridge lane + workspace port command.
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
type TransferSurface = {
  id: string,
  source: string,
  target: string,
  required: boolean
};

const DEFAULT_POLICY_PATH = process.env.CORE_MIGRATION_BRIDGE_POLICY_PATH
  ? path.resolve(process.env.CORE_MIGRATION_BRIDGE_POLICY_PATH)
  : path.join(ROOT, 'config', 'core_migration_bridge_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/migration/core_migration_bridge.js run --to=<org/repo|url> [--workspace=<path|name>] [--apply=1|0] [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/migration/core_migration_bridge.js rollback [--migration-id=<id>] [--apply=1|0] [--approval-note="..."] [--policy=<path>]');
  console.log('  node systems/migration/core_migration_bridge.js status [--policy=<path>]');
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
    git: {
      update_remote: true,
      remote_name: 'origin',
      auto_init_target_repo: true
    },
    workspace: {
      default_parent: '..'
    },
    transfer_surfaces: [
      { id: 'config', source: 'config', target: 'config', required: true },
      { id: 'habits', source: 'habits', target: 'habits', required: false },
      { id: 'vault', source: 'secrets/vault', target: 'secrets/vault', required: false },
      { id: 'memory', source: 'memory', target: 'memory', required: true },
      { id: 'scientific_receipts', source: 'state/science', target: 'state/science', required: false },
      { id: 'scientific_mode_v4_receipts', source: 'state/science/scientific_mode_v4', target: 'state/science/scientific_mode_v4', required: false },
      { id: 'research_receipts', source: 'state/research', target: 'state/research', required: false }
    ],
    paths: {
      latest_path: 'state/migration/core_bridge/latest.json',
      receipts_path: 'state/migration/core_bridge/receipts.jsonl',
      checkpoints_root: 'state/migration/core_bridge/checkpoints',
      registry_path: 'state/migration/core_bridge/registry.json'
    }
  };
}

function normalizeSurface(raw: AnyObj): TransferSurface | null {
  if (!raw || typeof raw !== 'object') return null;
  const id = normalizeToken(raw.id || '', 80);
  const source = cleanText(raw.source || '', 320).replace(/^\/+/, '');
  const target = cleanText(raw.target || '', 320).replace(/^\/+/, '');
  if (!id || !source || !target) return null;
  return {
    id,
    source,
    target,
    required: raw.required !== false
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const signing = raw.signing && typeof raw.signing === 'object' ? raw.signing : {};
  const git = raw.git && typeof raw.git === 'object' ? raw.git : {};
  const workspace = raw.workspace && typeof raw.workspace === 'object' ? raw.workspace : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};

  const surfaces = Array.isArray(raw.transfer_surfaces)
    ? raw.transfer_surfaces.map((row: AnyObj) => normalizeSurface(row)).filter(Boolean)
    : base.transfer_surfaces.map((row: AnyObj) => normalizeSurface(row)).filter(Boolean);

  return {
    version: cleanText(raw.version || base.version, 32),
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, base.strict_default),
    signing: {
      key_env: cleanText(signing.key_env || base.signing.key_env, 120) || base.signing.key_env,
      default_key: cleanText(signing.default_key || base.signing.default_key, 240) || base.signing.default_key,
      algorithm: cleanText(signing.algorithm || base.signing.algorithm, 40) || base.signing.algorithm
    },
    git: {
      update_remote: toBool(git.update_remote, base.git.update_remote),
      remote_name: cleanText(git.remote_name || base.git.remote_name, 40) || base.git.remote_name,
      auto_init_target_repo: toBool(git.auto_init_target_repo, base.git.auto_init_target_repo)
    },
    workspace: {
      default_parent: cleanText(workspace.default_parent || base.workspace.default_parent, 240) || base.workspace.default_parent
    },
    transfer_surfaces: surfaces.length ? surfaces : base.transfer_surfaces,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path),
      checkpoints_root: resolvePath(paths.checkpoints_root, base.paths.checkpoints_root),
      registry_path: resolvePath(paths.registry_path, base.paths.registry_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function repoSlugFromTarget(target: string) {
  const cleaned = String(target || '').trim().replace(/\.git$/i, '');
  if (!cleaned) return null;
  const httpMatch = cleaned.match(/github\.com[/:]([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)$/i);
  if (httpMatch) return httpMatch[1];
  const scpMatch = cleaned.match(/^git@[^:]+:([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)$/i);
  if (scpMatch) return scpMatch[1];
  const slugMatch = cleaned.match(/^([A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+)$/);
  if (slugMatch) return slugMatch[1];
  return null;
}

function normalizeTargetRepository(toRaw: string) {
  const cleaned = cleanText(toRaw || '', 240);
  const slug = repoSlugFromTarget(cleaned);
  if (slug) return {
    input: cleaned,
    slug,
    remote_url: `https://github.com/${slug}.git`
  };
  return {
    input: cleaned,
    slug: null,
    remote_url: cleaned
  };
}

function workspaceNameFromTarget(toRaw: string) {
  const normalized = normalizeTargetRepository(toRaw);
  if (normalized.slug) {
    const parts = normalized.slug.split('/');
    return parts[1] || 'protheus-workspace';
  }
  const tail = String(normalized.remote_url || '').split('/').filter(Boolean).pop() || 'protheus-workspace';
  return tail.replace(/\.git$/i, '') || 'protheus-workspace';
}

function resolveWorkspacePath(workspaceRaw: unknown, toRaw: string, policy: AnyObj) {
  const given = cleanText(workspaceRaw || '', 260);
  if (given) {
    return path.isAbsolute(given)
      ? path.resolve(given)
      : path.resolve(path.dirname(ROOT), given);
  }
  const defaultParent = path.resolve(ROOT, policy.workspace.default_parent || '..');
  return path.join(defaultParent, workspaceNameFromTarget(toRaw));
}

function runGit(cwd: string, argv: string[]) {
  const res = spawnSync('git', ['-C', cwd, ...argv], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  return {
    status: Number.isFinite(res.status) ? Number(res.status) : 1,
    stdout: String(res.stdout || '').trim(),
    stderr: String(res.stderr || '').trim()
  };
}

function gatherGitState(workspacePath: string, remoteName = 'origin') {
  const exists = fs.existsSync(path.join(workspacePath, '.git'));
  if (!exists) {
    return {
      git_present: false,
      remote_name: remoteName,
      remote_url: null,
      branch: null,
      head: null
    };
  }
  const remote = runGit(workspacePath, ['remote', 'get-url', remoteName]);
  const branch = runGit(workspacePath, ['rev-parse', '--abbrev-ref', 'HEAD']);
  const head = runGit(workspacePath, ['rev-parse', 'HEAD']);
  return {
    git_present: true,
    remote_name: remoteName,
    remote_url: remote.status === 0 ? remote.stdout : null,
    branch: branch.status === 0 ? branch.stdout : null,
    head: head.status === 0 ? head.stdout : null
  };
}

function isDirectory(filePath: string) {
  try {
    return fs.statSync(filePath).isDirectory();
  } catch {
    return false;
  }
}

function listFilesRecursive(rootPath: string) {
  const out: string[] = [];
  if (!fs.existsSync(rootPath)) return out;
  const st = fs.statSync(rootPath);
  if (st.isFile()) return [rootPath];
  const stack = [rootPath];
  while (stack.length) {
    const cur = stack.pop() as string;
    let entries: any[] = [];
    try {
      entries = fs.readdirSync(cur, { withFileTypes: true });
    } catch {
      entries = [];
    }
    entries.forEach((ent) => {
      const abs = path.join(cur, ent.name);
      if (ent.isDirectory()) stack.push(abs);
      else if (ent.isFile()) out.push(abs);
    });
  }
  return out.sort((a, b) => a.localeCompare(b));
}

function ensureDir(fileOrDir: string, treatAsFile = false) {
  const dir = treatAsFile ? path.dirname(fileOrDir) : fileOrDir;
  fs.mkdirSync(dir, { recursive: true });
}

function copyFile(src: string, dst: string) {
  ensureDir(dst, true);
  fs.copyFileSync(src, dst);
}

function removeFileAndEmptyParents(filePath: string, floorDir: string) {
  if (!fs.existsSync(filePath)) return;
  const st = fs.statSync(filePath);
  if (st.isDirectory()) {
    fs.rmSync(filePath, { recursive: true, force: true });
    return;
  }
  fs.unlinkSync(filePath);
  let cur = path.dirname(filePath);
  const floor = path.resolve(floorDir);
  for (let i = 0; i < 12; i += 1) {
    if (path.resolve(cur) === floor) break;
    let entries: string[] = [];
    try {
      entries = fs.readdirSync(cur);
    } catch {
      break;
    }
    if (entries.length > 0) break;
    try {
      fs.rmdirSync(cur);
    } catch {
      break;
    }
    cur = path.dirname(cur);
  }
}

function loadRegistry(policy: AnyObj) {
  return readJson(policy.paths.registry_path, {
    schema_version: '1.0',
    latest_migration_id: null,
    migrations: {}
  });
}

function saveRegistry(policy: AnyObj, registry: AnyObj) {
  registry.updated_at = nowIso();
  writeJsonAtomic(policy.paths.registry_path, registry);
}

function sign(policy: AnyObj, payload: AnyObj) {
  const envName = cleanText(policy.signing.key_env || 'PROTHEUS_MIGRATION_SIGNING_KEY', 120) || 'PROTHEUS_MIGRATION_SIGNING_KEY';
  const secret = cleanText(process.env[envName] || policy.signing.default_key || 'migration_dev_key', 400) || 'migration_dev_key';
  const canonical = JSON.stringify(payload);
  return {
    algorithm: cleanText(policy.signing.algorithm || 'sha256', 40) || 'sha256',
    key_id: stableHash(`${envName}:${secret}`, 12),
    signature: stableHash(`${canonical}|${secret}`, 48)
  };
}

function writeReceipt(policy: AnyObj, payload: AnyObj) {
  const row = {
    ts: nowIso(),
    schema_id: 'core_migration_bridge_receipt',
    schema_version: '1.0',
    ...payload
  };
  row.signature = sign(policy, {
    ts: row.ts,
    type: row.type,
    migration_id: row.migration_id || null,
    result: row.result || null,
    status: row.status || null,
    ok: row.ok === true
  });
  writeJsonAtomic(policy.paths.latest_path, row);
  appendJsonl(policy.paths.receipts_path, row);
  return row;
}

function buildTransferPlan(policy: AnyObj, sourceRoot: string, targetRoot: string) {
  const missingRequired: string[] = [];
  const surfaces: AnyObj[] = [];
  let fileCount = 0;
  let bytesTotal = 0;

  (policy.transfer_surfaces || []).forEach((surface: TransferSurface) => {
    const sourceAbs = path.join(sourceRoot, surface.source);
    const exists = fs.existsSync(sourceAbs);
    const files: AnyObj[] = [];
    if (!exists) {
      if (surface.required) missingRequired.push(surface.id);
      surfaces.push({
        id: surface.id,
        source: surface.source,
        target: surface.target,
        required: !!surface.required,
        exists: false,
        file_count: 0,
        bytes_total: 0,
        files: []
      });
      return;
    }

    const sourceFiles = listFilesRecursive(sourceAbs);
    sourceFiles.forEach((absPath) => {
      const relInSurface = path.relative(sourceAbs, absPath).replace(/\\/g, '/');
      const relativeTarget = path.join(surface.target, relInSurface).replace(/\\/g, '/');
      const destinationAbs = path.join(targetRoot, relativeTarget);
      let size = 0;
      try { size = Number(fs.statSync(absPath).size || 0); } catch {}
      fileCount += 1;
      bytesTotal += size;
      files.push({
        source_rel: path.join(surface.source, relInSurface).replace(/\\/g, '/'),
        source_abs: absPath,
        target_rel: relativeTarget,
        target_abs: destinationAbs,
        bytes: size,
        target_exists: fs.existsSync(destinationAbs)
      });
    });

    surfaces.push({
      id: surface.id,
      source: surface.source,
      target: surface.target,
      required: !!surface.required,
      exists: true,
      file_count: files.length,
      bytes_total: files.reduce((acc, row) => acc + Number(row.bytes || 0), 0),
      files
    });
  });

  return {
    missing_required_surfaces: missingRequired,
    surfaces,
    summary: {
      surface_count: surfaces.length,
      file_count: fileCount,
      bytes_total: bytesTotal
    }
  };
}

function runMigration(args: AnyObj, policy: AnyObj) {
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const apply = toBool(args.apply, false);
  const toRaw = cleanText(args.to || '', 260);
  if (!toRaw) {
    return writeReceipt(policy, {
      ok: false,
      type: 'core_migration_bridge_run',
      error: 'to_required',
      strict,
      apply
    });
  }

  const normalized = normalizeTargetRepository(toRaw);
  const targetWorkspace = resolveWorkspacePath(args.workspace, toRaw, policy);
  if (path.resolve(targetWorkspace) === path.resolve(ROOT)) {
    return writeReceipt(policy, {
      ok: false,
      type: 'core_migration_bridge_run',
      error: 'target_workspace_matches_source',
      strict,
      apply,
      target_workspace: targetWorkspace
    });
  }

  const migrationId = cleanText(
    args['migration-id'] || args.migration_id || `migr_${Date.now()}_${stableHash(`${normalized.remote_url}|${targetWorkspace}`, 10)}`,
    120
  );

  const checkpointDir = path.join(policy.paths.checkpoints_root, migrationId);
  const checkpointPath = path.join(checkpointDir, 'checkpoint.json');
  const targetPreexistingRoot = path.join(checkpointDir, 'target_preexisting');
  const transfer = buildTransferPlan(policy, ROOT, targetWorkspace);
  const sourceGit = gatherGitState(ROOT, policy.git.remote_name);
  const targetGitBefore = gatherGitState(targetWorkspace, policy.git.remote_name);
  const now = nowIso();

  const out: AnyObj = {
    ok: transfer.missing_required_surfaces.length === 0,
    type: 'core_migration_bridge_run',
    lane_id: 'V4-MIGR-001',
    migration_id: migrationId,
    strict,
    apply,
    source_workspace: ROOT,
    target_workspace: targetWorkspace,
    repository_target: {
      input: normalized.input,
      slug: normalized.slug,
      remote_url: normalized.remote_url
    },
    plan: {
      summary: transfer.summary,
      missing_required_surfaces: transfer.missing_required_surfaces,
      surfaces: transfer.surfaces.map((surface: AnyObj) => ({
        id: surface.id,
        source: surface.source,
        target: surface.target,
        required: surface.required,
        exists: surface.exists,
        file_count: surface.file_count,
        bytes_total: surface.bytes_total
      }))
    },
    checkpoint_path: rel(checkpointPath),
    source_git: sourceGit,
    target_git_before: targetGitBefore,
    applied: false,
    touched_files: [],
    backed_up_target_files: [],
    errors: []
  };

  if (strict && transfer.missing_required_surfaces.length > 0) {
    out.ok = false;
    out.error = 'missing_required_surfaces';
    return writeReceipt(policy, out);
  }

  if (!apply) {
    out.result = 'planned';
    return writeReceipt(policy, out);
  }

  try {
    ensureDir(targetWorkspace, false);
    ensureDir(checkpointDir, false);
    ensureDir(targetPreexistingRoot, false);

    const touched: string[] = [];
    const backedUp: string[] = [];
    const backedUpSet = new Set<string>();

    transfer.surfaces.forEach((surface: AnyObj) => {
      (surface.files || []).forEach((fileRow: AnyObj) => {
        const srcAbs = fileRow.source_abs;
        const dstAbs = fileRow.target_abs;
        const targetRel = cleanText(fileRow.target_rel || '', 400);

        if (fs.existsSync(dstAbs) && !backedUpSet.has(targetRel)) {
          const backupAbs = path.join(targetPreexistingRoot, targetRel);
          copyFile(dstAbs, backupAbs);
          backedUp.push(targetRel);
          backedUpSet.add(targetRel);
        }

        copyFile(srcAbs, dstAbs);
        touched.push(targetRel);
      });
    });

    let targetGitAfter = gatherGitState(targetWorkspace, policy.git.remote_name);
    let remoteAction = 'skipped';
    if (policy.git.update_remote && normalized.remote_url) {
      if (!targetGitAfter.git_present && policy.git.auto_init_target_repo) {
        const initRun = runGit(targetWorkspace, ['init']);
        if (initRun.status !== 0) {
          throw new Error(`git_init_failed:${initRun.stderr || initRun.stdout || 'unknown'}`);
        }
      }

      const postInit = gatherGitState(targetWorkspace, policy.git.remote_name);
      const existingRemote = postInit.remote_url;
      if (existingRemote) {
        const setRun = runGit(targetWorkspace, ['remote', 'set-url', policy.git.remote_name, normalized.remote_url]);
        if (setRun.status !== 0) {
          throw new Error(`git_remote_set_failed:${setRun.stderr || setRun.stdout || 'unknown'}`);
        }
        remoteAction = 'set-url';
      } else {
        const addRun = runGit(targetWorkspace, ['remote', 'add', policy.git.remote_name, normalized.remote_url]);
        if (addRun.status !== 0) {
          throw new Error(`git_remote_add_failed:${addRun.stderr || addRun.stdout || 'unknown'}`);
        }
        remoteAction = 'add';
      }
      targetGitAfter = gatherGitState(targetWorkspace, policy.git.remote_name);
    }

    const checkpoint = {
      schema_id: 'core_migration_bridge_checkpoint',
      schema_version: '1.0',
      ts: now,
      migration_id: migrationId,
      source_workspace: ROOT,
      target_workspace: targetWorkspace,
      repository_target: normalized,
      source_git: sourceGit,
      target_git_before: targetGitBefore,
      target_git_after: targetGitAfter,
      remote_action: remoteAction,
      touched_files: touched,
      backed_up_target_files: backedUp,
      transfer_summary: transfer.summary,
      missing_required_surfaces: transfer.missing_required_surfaces,
      strict,
      apply
    };
    writeJsonAtomic(checkpointPath, checkpoint);

    const registry = loadRegistry(policy);
    registry.migrations = registry.migrations && typeof registry.migrations === 'object' ? registry.migrations : {};
    registry.latest_migration_id = migrationId;
    registry.migrations[migrationId] = {
      ts: now,
      migration_id: migrationId,
      source_workspace: ROOT,
      target_workspace: targetWorkspace,
      repository_target: normalized,
      checkpoint_path: rel(checkpointPath),
      status: 'applied',
      rolled_back: false,
      touched_files_count: touched.length
    };
    saveRegistry(policy, registry);

    out.applied = true;
    out.result = 'applied';
    out.ok = transfer.missing_required_surfaces.length === 0;
    out.touched_files = touched;
    out.backed_up_target_files = backedUp;
    out.target_git_after = targetGitAfter;
    out.remote_action = remoteAction;
  } catch (err) {
    out.ok = false;
    out.result = 'failed';
    out.errors.push(cleanText((err as Error).message || String(err), 260));
  }

  return writeReceipt(policy, out);
}

function rollbackMigration(args: AnyObj, policy: AnyObj) {
  const apply = toBool(args.apply, false);
  const strict = args.strict != null ? toBool(args.strict, false) : policy.strict_default;
  const approvalNote = cleanText(args['approval-note'] || args.approval_note || '', 400);

  const registry = loadRegistry(policy);
  const migrationId = cleanText(args['migration-id'] || args.migration_id || registry.latest_migration_id || '', 120);
  if (!migrationId) {
    return writeReceipt(policy, {
      ok: false,
      type: 'core_migration_bridge_rollback',
      error: 'migration_id_required',
      strict,
      apply
    });
  }

  const checkpointPath = path.join(policy.paths.checkpoints_root, migrationId, 'checkpoint.json');
  const checkpoint = readJson(checkpointPath, null);
  if (!checkpoint || typeof checkpoint !== 'object') {
    return writeReceipt(policy, {
      ok: false,
      type: 'core_migration_bridge_rollback',
      migration_id: migrationId,
      error: 'checkpoint_not_found',
      checkpoint_path: rel(checkpointPath),
      strict,
      apply
    });
  }

  const targetWorkspace = cleanText(checkpoint.target_workspace || '', 400);
  const targetPreexistingRoot = path.join(policy.paths.checkpoints_root, migrationId, 'target_preexisting');
  const touched = Array.isArray(checkpoint.touched_files) ? checkpoint.touched_files.map((v: unknown) => cleanText(v, 400)).filter(Boolean) : [];
  const backedUp = new Set(
    Array.isArray(checkpoint.backed_up_target_files)
      ? checkpoint.backed_up_target_files.map((v: unknown) => cleanText(v, 400)).filter(Boolean)
      : []
  );

  const out: AnyObj = {
    ok: true,
    type: 'core_migration_bridge_rollback',
    lane_id: 'V4-MIGR-001',
    migration_id: migrationId,
    strict,
    apply,
    checkpoint_path: rel(checkpointPath),
    target_workspace: targetWorkspace,
    approval_note_present: !!approvalNote,
    restored_files: [],
    removed_files: [],
    errors: []
  };

  if (apply && !approvalNote) {
    out.ok = false;
    out.error = 'approval_note_required_for_apply';
    return writeReceipt(policy, out);
  }

  if (!apply) {
    out.result = 'planned';
    out.restore_plan_count = touched.length;
    return writeReceipt(policy, out);
  }

  try {
    touched.forEach((targetRel: string) => {
      const absTarget = path.join(targetWorkspace, targetRel);
      const backupAbs = path.join(targetPreexistingRoot, targetRel);
      if (backedUp.has(targetRel) && fs.existsSync(backupAbs)) {
        copyFile(backupAbs, absTarget);
        out.restored_files.push(targetRel);
      } else {
        removeFileAndEmptyParents(absTarget, targetWorkspace);
        out.removed_files.push(targetRel);
      }
    });

    const before = checkpoint.target_git_before && typeof checkpoint.target_git_before === 'object'
      ? checkpoint.target_git_before
      : {};
    const remoteName = cleanText(before.remote_name || policy.git.remote_name || 'origin', 40) || 'origin';
    const beforeRemote = cleanText(before.remote_url || '', 300);

    if (beforeRemote && fs.existsSync(path.join(targetWorkspace, '.git'))) {
      const remoteState = runGit(targetWorkspace, ['remote', 'get-url', remoteName]);
      if (remoteState.status === 0) {
        const setRun = runGit(targetWorkspace, ['remote', 'set-url', remoteName, beforeRemote]);
        if (setRun.status !== 0) throw new Error(`git_remote_restore_failed:${setRun.stderr || setRun.stdout || 'unknown'}`);
      } else {
        const addRun = runGit(targetWorkspace, ['remote', 'add', remoteName, beforeRemote]);
        if (addRun.status !== 0) throw new Error(`git_remote_restore_add_failed:${addRun.stderr || addRun.stdout || 'unknown'}`);
      }
      out.remote_restored = true;
      out.remote_name = remoteName;
      out.remote_url = beforeRemote;
    } else {
      out.remote_restored = false;
    }

    const registryRecord = registry.migrations && registry.migrations[migrationId]
      ? registry.migrations[migrationId]
      : null;
    if (registryRecord) {
      registryRecord.rolled_back = true;
      registryRecord.status = 'rolled_back';
      registryRecord.rollback_ts = nowIso();
      registryRecord.rollback_approval_note = approvalNote;
      saveRegistry(policy, registry);
    }

    out.result = 'applied';
  } catch (err) {
    out.ok = false;
    out.result = 'failed';
    out.errors.push(cleanText((err as Error).message || String(err), 260));
  }

  return writeReceipt(policy, out);
}

function status(policy: AnyObj) {
  const latest = readJson(policy.paths.latest_path, null);
  const registry = loadRegistry(policy);
  return {
    ok: true,
    type: 'core_migration_bridge_status',
    lane_id: 'V4-MIGR-001',
    enabled: policy.enabled,
    policy_path: rel(policy.policy_path),
    latest_path: rel(policy.paths.latest_path),
    receipts_path: rel(policy.paths.receipts_path),
    checkpoints_root: rel(policy.paths.checkpoints_root),
    latest,
    latest_migration_id: registry.latest_migration_id || null,
    registry_count: registry.migrations && typeof registry.migrations === 'object'
      ? Object.keys(registry.migrations).length
      : 0
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
  if (!policy.enabled) emit({ ok: false, error: 'core_migration_bridge_disabled' }, 1);

  if (cmd === 'run') {
    const out = runMigration(args, policy);
    emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'rollback') {
    const out = rollbackMigration(args, policy);
    emit(out, out.ok ? 0 : 1);
  }
  if (cmd === 'status') emit(status(policy));

  usage();
  process.exit(1);
}

main();
