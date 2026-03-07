#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  toBool,
  ensureDir,
  writeJsonAtomic,
  appendJsonl,
  emit,
  relPath
} = require('../../lib/queued_backlog_runtime');
const { CANONICAL_PATHS } = require('../../lib/runtime_path_registry');

const CLIENT_ROOT = path.join(ROOT, 'client');
const CORE_ROOT = path.join(ROOT, 'core');
const CLIENT_LOCAL = path.join(ROOT, CANONICAL_PATHS.client_local_root);
const CORE_LOCAL = path.join(ROOT, CANONICAL_PATHS.core_local_root);
const STATE_ROOT = path.join(CLIENT_LOCAL, 'state', 'ops', 'migrate_to_planes');
const LATEST_PATH = path.join(STATE_ROOT, 'latest.json');
const RECEIPTS_PATH = path.join(STATE_ROOT, 'receipts.jsonl');
const MIGRATIONS_ROOT = path.join(STATE_ROOT, 'migrations');

type Mapping = {
  id: string;
  source: string;
  target: string;
  default_mode: 'copy' | 'move';
  allow_tracked_move: boolean;
  notes: string;
};

const ROOT_RUNTIME_NAMES = [
  'adaptive',
  'memory',
  'habits',
  'logs',
  'patches',
  'reports',
  'research',
  'secrets',
  'config'
];

const MAPPINGS: Mapping[] = [
  {
    id: 'root_state',
    source: path.join(ROOT, 'state'),
    target: path.join(CLIENT_LOCAL, 'state'),
    default_mode: 'move',
    allow_tracked_move: false,
    notes: 'legacy runtime state mirror'
  },
  {
    id: 'root_local_state',
    source: path.join(ROOT, 'local', 'state'),
    target: path.join(CLIENT_LOCAL, 'state'),
    default_mode: 'move',
    allow_tracked_move: false,
    notes: 'legacy root local state mirror'
  },
  {
    id: 'root_private_lenses',
    source: path.join(ROOT, '.private-lenses'),
    target: path.join(CLIENT_LOCAL, 'private-lenses'),
    default_mode: 'move',
    allow_tracked_move: false,
    notes: 'private lens config surface'
  },
  {
    id: 'client_logs',
    source: path.join(CLIENT_ROOT, 'logs'),
    target: path.join(CLIENT_LOCAL, 'logs'),
    default_mode: 'copy',
    allow_tracked_move: false,
    notes: 'client runtime logs'
  },
  {
    id: 'client_secrets',
    source: path.join(CLIENT_ROOT, 'secrets'),
    target: path.join(CLIENT_LOCAL, 'secrets'),
    default_mode: 'copy',
    allow_tracked_move: false,
    notes: 'client local secrets'
  },
  {
    id: 'core_state',
    source: path.join(CORE_ROOT, 'state'),
    target: path.join(CORE_LOCAL, 'state'),
    default_mode: 'move',
    allow_tracked_move: false,
    notes: 'core runtime state mirror'
  },
  {
    id: 'core_memory_legacy',
    source: path.join(CORE_ROOT, 'memory'),
    target: path.join(CORE_LOCAL, 'memory'),
    default_mode: 'move',
    allow_tracked_move: false,
    notes: 'legacy core memory runtime surface'
  }
].concat(ROOT_RUNTIME_NAMES.map((name) => ({
  id: `root_runtime_${name}`,
  source: path.join(ROOT, name),
  target: path.join(CLIENT_LOCAL, name === 'config' ? 'config' : name),
  default_mode: 'move' as const,
  allow_tracked_move: false,
  notes: 'legacy root runtime folder'
})));

function usage() {
  console.log('Usage:');
  console.log('  node client/systems/ops/migrate_to_planes.js run [--apply=0|1] [--move-untracked=1|0] [--include-missing=0|1] [--compat-symlinks=1|0 (deprecated, default 0)]');
  console.log('  node client/systems/ops/migrate_to_planes.js rollback --id=<migration_id|latest> [--prune-copies=1|0]');
  console.log('  node client/systems/ops/migrate_to_planes.js status');
  console.log('  node client/systems/ops/migrate_to_planes.js plan');
}

function listEntries(absDir: string) {
  if (!fs.existsSync(absDir)) return [];
  return fs.readdirSync(absDir, { withFileTypes: true }).map((entry: any) => ({
    name: entry.name,
    abs: path.join(absDir, entry.name),
    dir: entry.isDirectory(),
    file: entry.isFile(),
    sym: entry.isSymbolicLink()
  }));
}

function countTree(absDir: string) {
  const summary = { files: 0, dirs: 0, bytes: 0 };
  if (!fs.existsSync(absDir)) return summary;
  const stack = [absDir];
  while (stack.length) {
    const cursor = stack.pop() as string;
    const entries = listEntries(cursor);
    for (const entry of entries) {
      if (entry.dir) {
        summary.dirs += 1;
        stack.push(entry.abs);
      } else if (entry.file) {
        summary.files += 1;
        try {
          summary.bytes += Number(fs.statSync(entry.abs).size || 0);
        } catch {}
      }
    }
  }
  return summary;
}

function trackedCount(absPath: string) {
  const rel = path.relative(ROOT, absPath).replace(/\\/g, '/');
  if (!rel || rel.startsWith('..')) return 0;
  const out = spawnSync('git', ['ls-files', '--', rel], {
    cwd: ROOT,
    encoding: 'utf8'
  });
  const rows = String(out.stdout || '').split('\n').map((r) => r.trim()).filter(Boolean);
  return rows.length;
}

function ensureParent(absPath: string) {
  ensureDir(path.dirname(absPath));
}

function migrationId() {
  const stamp = nowIso().replace(/[^0-9]/g, '').slice(0, 14);
  const rand = Math.random().toString(36).slice(2, 10);
  return `mtp_${stamp}_${rand}`;
}

function relFromRoot(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function absFromRoot(relValue: string) {
  const clean = cleanText(relValue || '', 520).replace(/\\/g, '/');
  if (!clean || clean === '.') return ROOT;
  return path.join(ROOT, clean);
}

function safeLstat(absPath: string) {
  try {
    return fs.lstatSync(absPath);
  } catch {
    return null;
  }
}

function ensureCompatSymlink(sourceAbs: string, targetAbs: string) {
  const srcStat = safeLstat(sourceAbs);
  if (srcStat && !srcStat.isSymbolicLink()) {
    return { created: false, reason: 'source_exists' };
  }
  if (srcStat && srcStat.isSymbolicLink()) {
    try {
      const existing = fs.realpathSync(sourceAbs);
      const targetReal = fs.realpathSync(targetAbs);
      if (existing === targetReal) return { created: false, reason: 'already_linked' };
      return { created: false, reason: 'conflicting_link' };
    } catch {
      return { created: false, reason: 'link_unresolved' };
    }
  }
  if (!fs.existsSync(targetAbs)) return { created: false, reason: 'target_missing' };
  ensureParent(sourceAbs);
  fs.symlinkSync(targetAbs, sourceAbs);
  return { created: true, reason: null };
}

function removeCompatSymlink(sourceAbs: string, targetAbs: string) {
  const srcStat = safeLstat(sourceAbs);
  if (!srcStat || !srcStat.isSymbolicLink()) return { removed: false, reason: 'not_symlink' };
  try {
    const existing = fs.realpathSync(sourceAbs);
    const targetReal = fs.realpathSync(targetAbs);
    if (existing !== targetReal) return { removed: false, reason: 'different_target' };
  } catch {
    return { removed: false, reason: 'link_unresolved' };
  }
  try {
    fs.unlinkSync(sourceAbs);
    return { removed: true, reason: null };
  } catch (err: any) {
    return {
      removed: false,
      reason: cleanText(err && err.message ? err.message : err, 220) || 'unlink_failed'
    };
  }
}

function removeTree(absPath: string) {
  try {
    fs.rmSync(absPath, { recursive: true, force: true });
  } catch {}
}

function removeEmptyDir(absPath: string) {
  try {
    const stat = fs.lstatSync(absPath);
    if (!stat.isDirectory()) return;
    const entries = fs.readdirSync(absPath);
    if (entries.length === 0) fs.rmdirSync(absPath);
  } catch {}
}

function copyTree(src: string, dest: string) {
  ensureDir(dest);
  const entries = listEntries(src);
  for (const entry of entries) {
    const nextDest = path.join(dest, entry.name);
    if (entry.dir) {
      copyTree(entry.abs, nextDest);
      continue;
    }
    if (entry.file) {
      ensureParent(nextDest);
      if (!fs.existsSync(nextDest)) {
        fs.copyFileSync(entry.abs, nextDest);
      }
      continue;
    }
    if (entry.sym) {
      try {
        const link = fs.readlinkSync(entry.abs);
        if (!fs.existsSync(nextDest)) fs.symlinkSync(link, nextDest);
      } catch {}
    }
  }
}

function moveTree(src: string, dest: string) {
  ensureDir(dest);
  const entries = listEntries(src);
  for (const entry of entries) {
    const nextDest = path.join(dest, entry.name);
    if (entry.dir) {
      moveTree(entry.abs, nextDest);
      try {
        const rest = fs.readdirSync(entry.abs);
        if (rest.length === 0) fs.rmdirSync(entry.abs);
      } catch {}
      continue;
    }
    if (entry.file || entry.sym) {
      ensureParent(nextDest);
      if (!fs.existsSync(nextDest)) {
        try {
          fs.renameSync(entry.abs, nextDest);
          continue;
        } catch {}
      }
      try {
        if (entry.file) {
          fs.copyFileSync(entry.abs, nextDest);
        } else if (entry.sym) {
          const link = fs.readlinkSync(entry.abs);
          if (!fs.existsSync(nextDest)) fs.symlinkSync(link, nextDest);
        }
      } catch {}
      try { fs.rmSync(entry.abs, { force: true }); } catch {}
    }
  }
}

function mkdirBlueprint() {
  const dirs = [
    path.join(CLIENT_LOCAL, 'adaptive'),
    path.join(CLIENT_LOCAL, 'memory'),
    path.join(CLIENT_LOCAL, 'logs'),
    path.join(CLIENT_LOCAL, 'secrets'),
    path.join(CLIENT_LOCAL, 'reports'),
    path.join(CLIENT_LOCAL, 'research'),
    path.join(CLIENT_LOCAL, 'patches'),
    path.join(CLIENT_LOCAL, 'config'),
    path.join(CLIENT_LOCAL, 'private-lenses'),
    path.join(CLIENT_LOCAL, 'habits'),
    path.join(CLIENT_LOCAL, 'state'),
    path.join(CORE_LOCAL, 'state'),
    path.join(CORE_LOCAL, 'memory'),
    path.join(CORE_LOCAL, 'logs'),
    path.join(CORE_LOCAL, 'config'),
    path.join(CORE_LOCAL, 'cache'),
    path.join(CORE_LOCAL, 'device')
  ];
  for (const dir of dirs) {
    ensureDir(dir);
    const gitkeep = path.join(dir, '.gitkeep');
    if (!fs.existsSync(gitkeep)) fs.writeFileSync(gitkeep, '', 'utf8');
  }
}

function planRows(includeMissing: boolean) {
  return MAPPINGS
    .map((row) => {
      const exists = fs.existsSync(row.source);
      if (!exists && !includeMissing) return null;
      const tracked = exists ? trackedCount(row.source) : 0;
      const summary = exists ? countTree(row.source) : { files: 0, dirs: 0, bytes: 0 };
      return {
        ...row,
        source_rel: relPath(row.source),
        target_rel: relPath(row.target),
        source_exists: exists,
        tracked_files: tracked,
        summary,
        recommended_mode: tracked > 0 && !row.allow_tracked_move ? 'copy' : row.default_mode
      };
    })
    .filter(Boolean);
}

function runMigration(args: Record<string, any>) {
  const apply = toBool(args.apply, false);
  const moveUntracked = toBool(args['move-untracked'], true);
  const includeMissing = toBool(args['include-missing'], false);
  const compatSymlinks = toBool(args['compat-symlinks'], false);
  const migration_id = apply ? migrationId() : null;

  mkdirBlueprint();
  ensureDir(MIGRATIONS_ROOT);
  const rows: any[] = planRows(includeMissing);

  const results = rows.map((row) => {
    const mode = row.recommended_mode;
    const shouldMove = mode === 'move' && moveUntracked;
    let action = 'skipped';
    let reason = null;

    const sourceAbs = absFromRoot(row.source_rel);
    const targetAbs = absFromRoot(row.target_rel);
    const targetExistedBefore = fs.existsSync(targetAbs);
    let compatSymlink = { created: false, reason: null as string | null };

    if (!row.source_exists) {
      reason = 'source_missing';
      return {
        ...row,
        source_abs: sourceAbs,
        target_abs: targetAbs,
        target_existed_before: targetExistedBefore,
        compat_symlink_created: false,
        compat_symlink_reason: reason,
        action,
        reason
      };
    }

    if (!apply) {
      action = shouldMove ? 'planned_move' : 'planned_copy';
      return {
        ...row,
        source_abs: sourceAbs,
        target_abs: targetAbs,
        target_existed_before: targetExistedBefore,
        compat_symlink_created: false,
        compat_symlink_reason: null,
        action,
        reason
      };
    }

    try {
      if (shouldMove) {
        moveTree(sourceAbs, targetAbs);
        removeEmptyDir(sourceAbs);
        action = 'moved';
        if (compatSymlinks) {
          compatSymlink = ensureCompatSymlink(sourceAbs, targetAbs);
        }
      } else {
        copyTree(sourceAbs, targetAbs);
        action = 'copied';
      }
    } catch (err: any) {
      action = 'failed';
      reason = cleanText(err && err.message ? err.message : err, 220) || 'migration_failed';
    }

    return {
      ...row,
      source_abs: sourceAbs,
      target_abs: targetAbs,
      target_existed_before: targetExistedBefore,
      compat_symlink_created: !!compatSymlink.created,
      compat_symlink_reason: compatSymlink.reason,
      action,
      reason
    };
  });

  const migrated = results.filter((r) => r.action === 'moved' || r.action === 'copied').length;
  const failed = results.filter((r) => r.action === 'failed').length;

  const receipt = {
    ok: failed === 0,
    type: 'migrate_to_planes',
    ts: nowIso(),
    migration_id,
    apply,
    move_untracked: moveUntracked,
    include_missing: includeMissing,
    compat_symlinks: compatSymlinks,
    migrated,
    failed,
    rows: results
  };

  if (apply && migration_id) {
    const manifest = {
      schema_id: 'migrate_to_planes_manifest',
      schema_version: '1.0',
      migration_id,
      ts: receipt.ts,
      apply: true,
      move_untracked: moveUntracked,
      include_missing: includeMissing,
      compat_symlinks: compatSymlinks,
      rows: results.map((row) => ({
        id: row.id,
        source_rel: relFromRoot(row.source_abs),
        target_rel: relFromRoot(row.target_abs),
        action: row.action,
        reason: row.reason,
        target_existed_before: !!row.target_existed_before,
        compat_symlink_created: !!row.compat_symlink_created,
        compat_symlink_reason: row.compat_symlink_reason
      }))
    };
    const manifestPath = path.join(MIGRATIONS_ROOT, `${migration_id}.json`);
    writeJsonAtomic(manifestPath, manifest);
    (receipt as any).manifest_path = relPath(manifestPath);
  }

  writeJsonAtomic(LATEST_PATH, receipt);
  appendJsonl(RECEIPTS_PATH, receipt);
  return receipt;
}

function loadManifest(idToken: string) {
  const clean = cleanText(idToken || '', 120);
  if (!clean) return null;
  const manifestPath = path.join(MIGRATIONS_ROOT, `${clean}.json`);
  if (!fs.existsSync(manifestPath)) return null;
  try {
    const parsed = JSON.parse(fs.readFileSync(manifestPath, 'utf8'));
    return {
      manifest: parsed,
      path: manifestPath
    };
  } catch {
    return null;
  }
}

function rollback(args: Record<string, any>) {
  mkdirBlueprint();
  ensureDir(MIGRATIONS_ROOT);
  const idRaw = cleanText(args.id || args['migration-id'] || 'latest', 120) || 'latest';
  const pruneCopies = toBool(args['prune-copies'], false);

  let targetId = idRaw;
  if (idRaw === 'latest') {
    const latest = fs.existsSync(LATEST_PATH)
      ? JSON.parse(fs.readFileSync(LATEST_PATH, 'utf8'))
      : null;
    targetId = cleanText(latest && latest.migration_id, 120) || '';
  }

  if (!targetId) {
    return {
      ok: false,
      type: 'migrate_to_planes_rollback',
      ts: nowIso(),
      error: 'missing_migration_id'
    };
  }

  const loaded = loadManifest(targetId);
  if (!loaded) {
    return {
      ok: false,
      type: 'migrate_to_planes_rollback',
      ts: nowIso(),
      migration_id: targetId,
      error: 'manifest_not_found'
    };
  }

  const rows = Array.isArray(loaded.manifest.rows) ? loaded.manifest.rows.slice().reverse() : [];
  const results: any[] = [];

  for (const row of rows) {
    const sourceAbs = absFromRoot(row.source_rel);
    const targetAbs = absFromRoot(row.target_rel);
    let action = 'noop';
    let reason = null;
    let symlink_removed = false;

    try {
      if (row.compat_symlink_created) {
        const removed = removeCompatSymlink(sourceAbs, targetAbs);
        symlink_removed = !!removed.removed;
      }

      if (row.action === 'moved') {
        if (fs.existsSync(targetAbs)) {
          moveTree(targetAbs, sourceAbs);
          action = 'restored_move';
          if (fs.existsSync(targetAbs)) {
            const remaining = listEntries(targetAbs);
            if (remaining.length === 0) removeTree(targetAbs);
          }
        } else {
          action = 'missing_target';
          reason = 'target_missing_for_restore';
        }
      } else if (row.action === 'copied') {
        action = 'copied_noop';
        if (pruneCopies && row.target_existed_before === false && fs.existsSync(targetAbs)) {
          removeTree(targetAbs);
          action = 'pruned_copy_target';
        }
      }
    } catch (err: any) {
      action = 'failed';
      reason = cleanText(err && err.message ? err.message : err, 220) || 'rollback_failed';
    }

    results.push({
      id: cleanText(row.id, 80),
      source_rel: relFromRoot(sourceAbs),
      target_rel: relFromRoot(targetAbs),
      action,
      reason,
      symlink_removed
    });
  }

  const failed = results.filter((r) => r.action === 'failed').length;
  const out = {
    ok: failed === 0,
    type: 'migrate_to_planes_rollback',
    ts: nowIso(),
    migration_id: targetId,
    manifest_path: relPath(loaded.path),
    prune_copies: pruneCopies,
    failed,
    rows: results
  };

  writeJsonAtomic(LATEST_PATH, out);
  appendJsonl(RECEIPTS_PATH, out);
  return out;
}

function status() {
  const latest = fs.existsSync(LATEST_PATH)
    ? JSON.parse(fs.readFileSync(LATEST_PATH, 'utf8'))
    : null;
  return {
    ok: !!latest,
    type: 'migrate_to_planes_status',
    ts: nowIso(),
    latest,
    known_migrations: fs.existsSync(MIGRATIONS_ROOT)
      ? fs.readdirSync(MIGRATIONS_ROOT).filter((name: string) => name.endsWith('.json')).length
      : 0,
    local_roots: {
      client_local: relPath(CLIENT_LOCAL),
      core_local: relPath(CORE_LOCAL)
    }
  };
}

function plan(args: Record<string, any>) {
  mkdirBlueprint();
  return {
    ok: true,
    type: 'migrate_to_planes_plan',
    ts: nowIso(),
    include_missing: toBool(args['include-missing'], false),
    rows: planRows(toBool(args['include-missing'], false))
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 60).toLowerCase() || 'status';

  if (cmd === 'help' || cmd === '--help' || cmd === '-h' || args.help) {
    usage();
    return;
  }

  if (cmd === 'run') {
    const out = runMigration(args);
    emit(out, out.ok ? 0 : 1);
    return;
  }

  if (cmd === 'rollback') {
    const out = rollback(args);
    emit(out, out.ok ? 0 : 1);
    return;
  }

  if (cmd === 'status') {
    const out = status();
    emit(out, out.ok ? 0 : 1);
    return;
  }

  if (cmd === 'plan') {
    const out = plan(args);
    emit(out, 0);
    return;
  }

  emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
}

main();
