#!/usr/bin/env node

const fs = require('node:fs');
const path = require('node:path');
const THIS_FILE = __filename;
const THIS_DIR = __dirname;
const WORKSPACE_ROOT = path.resolve(THIS_DIR, '..', '..', '..', '..');
const CONTINUITY_FILES = [
  'SOUL.md',
  'USER.md',
  'HEARTBEAT.md',
  'IDENTITY.md',
  'TOOLS.md',
  'MEMORY.md',
];
const ROOT_DEPRECATED_FILES = [...CONTINUITY_FILES];
const RESET_CONFIRM = 'RESET_LOCAL';

function isoStamp() {
  return new Date().toISOString().replace(/[-:]/g, '').replace(/\.\d{3}Z$/, 'Z');
}

function ensureDir(dir) {
  fs.mkdirSync(dir, { recursive: true });
}

function copyFile(src, dst) {
  ensureDir(path.dirname(dst));
  fs.copyFileSync(src, dst);
}

function moveFile(src, dst) {
  ensureDir(path.dirname(dst));
  fs.renameSync(src, dst);
}

function parseArgValue(args, key) {
  const inline = args.find((arg) => arg.startsWith(`${key}=`));
  if (inline) return inline.slice(key.length + 1).trim();
  const idx = args.findIndex((arg) => arg === key);
  if (idx >= 0 && idx + 1 < args.length) {
    return String(args[idx + 1]).trim();
  }
  return '';
}

function workspacePaths(workspaceRoot) {
  const localWorkspace = path.join(workspaceRoot, 'local', 'workspace');
  return {
    workspaceRoot,
    templateDir: path.join(workspaceRoot, 'docs', 'workspace', 'templates', 'assistant'),
    localWorkspace,
    assistantDir: path.join(localWorkspace, 'assistant'),
    reportsDir: path.join(localWorkspace, 'reports'),
    memoryDir: path.join(localWorkspace, 'memory'),
    privateDir: path.join(localWorkspace, 'private'),
    archiveRoot: path.join(localWorkspace, 'archive'),
  };
}

function continuityStatus(workspaceRoot = WORKSPACE_ROOT) {
  const paths = workspacePaths(workspaceRoot);
  const assistant_files = CONTINUITY_FILES.map((name) => ({
    file: name,
    exists: fs.existsSync(path.join(paths.assistantDir, name)),
    template_exists: fs.existsSync(path.join(paths.templateDir, name)),
  }));
  return {
    ok: true,
    type: 'local_runtime_partitioner',
    command: 'status',
    workspace_root: workspaceRoot,
    assistant_dir: paths.assistantDir,
    templates_dir: paths.templateDir,
    assistant_files,
    deprecated_root_files: ROOT_DEPRECATED_FILES.filter((name) =>
      fs.existsSync(path.join(workspaceRoot, name))
    ),
  };
}

function ensureLocalWorkspaceStructure(paths) {
  ensureDir(paths.assistantDir);
  ensureDir(paths.reportsDir);
  ensureDir(paths.memoryDir);
  ensureDir(paths.privateDir);
  ensureDir(paths.archiveRoot);
}

function archiveDeprecatedRootContinuity(paths, migrateMissing = true) {
  const migrated = [];
  const archived = [];
  let archiveDir = '';
  for (const name of ROOT_DEPRECATED_FILES) {
    const rootPath = path.join(paths.workspaceRoot, name);
    if (!fs.existsSync(rootPath)) continue;
    const assistantPath = path.join(paths.assistantDir, name);
    if (migrateMissing && !fs.existsSync(assistantPath)) {
      moveFile(rootPath, assistantPath);
      migrated.push(name);
      continue;
    }
    if (!archiveDir) {
      archiveDir = path.join(paths.archiveRoot, `root-continuity-${isoStamp()}`);
      ensureDir(archiveDir);
    }
    moveFile(rootPath, path.join(archiveDir, name));
    archived.push(name);
  }
  return { migrated, archived, archive_dir: archiveDir || null };
}

function generateMissingContinuity(paths) {
  const generated = [];
  const missing_templates = [];
  for (const name of CONTINUITY_FILES) {
    const dst = path.join(paths.assistantDir, name);
    if (fs.existsSync(dst)) continue;
    const template = path.join(paths.templateDir, name);
    if (!fs.existsSync(template)) {
      missing_templates.push(name);
      continue;
    }
    copyFile(template, dst);
    generated.push(name);
  }
  return { generated, missing_templates };
}

function initLocalRuntime(workspaceRoot = WORKSPACE_ROOT) {
  const paths = workspacePaths(workspaceRoot);
  ensureLocalWorkspaceStructure(paths);
  const migrated = archiveDeprecatedRootContinuity(paths, true);
  const generated = generateMissingContinuity(paths);
  return {
    ok: generated.missing_templates.length === 0,
    type: 'local_runtime_partitioner',
    command: 'init',
    workspace_root: workspaceRoot,
    assistant_dir: paths.assistantDir,
    generated_files: generated.generated,
    migrated_root_files: migrated.migrated,
    archived_root_files: migrated.archived,
    archive_dir: migrated.archive_dir,
    missing_templates: generated.missing_templates,
  };
}

function resetLocalRuntime(args, workspaceRoot = WORKSPACE_ROOT) {
  const confirm = parseArgValue(args, '--confirm');
  if (confirm !== RESET_CONFIRM) {
    return {
      ok: false,
      type: 'local_runtime_partitioner',
      command: 'reset',
      error: 'missing_confirm_reset_local',
      required_confirm: RESET_CONFIRM,
    };
  }

  const paths = workspacePaths(workspaceRoot);
  ensureLocalWorkspaceStructure(paths);
  const resetArchive = path.join(paths.archiveRoot, `assistant-reset-${isoStamp()}`);
  ensureDir(resetArchive);
  const archived_assistant_files = [];
  for (const name of CONTINUITY_FILES) {
    const assistantPath = path.join(paths.assistantDir, name);
    if (!fs.existsSync(assistantPath)) continue;
    moveFile(assistantPath, path.join(resetArchive, name));
    archived_assistant_files.push(name);
  }
  const migrated = archiveDeprecatedRootContinuity(paths, false);
  const generated = generateMissingContinuity(paths);
  return {
    ok: generated.missing_templates.length === 0,
    type: 'local_runtime_partitioner',
    command: 'reset',
    workspace_root: workspaceRoot,
    assistant_dir: paths.assistantDir,
    assistant_archive_dir: resetArchive,
    archived_assistant_files,
    generated_files: generated.generated,
    migrated_root_files: migrated.migrated,
    archived_root_files: migrated.archived,
    archive_dir: migrated.archive_dir,
    missing_templates: generated.missing_templates,
  };
}

function run(argv = [], options = {}) {
  const args = Array.isArray(argv) ? argv.map((arg) => String(arg)) : [];
  const workspaceRoot = options.workspaceRoot
    ? path.resolve(String(options.workspaceRoot))
    : WORKSPACE_ROOT;
  const command = (args[0] || 'status').trim().toLowerCase();
  switch (command) {
    case 'init':
      return initLocalRuntime(workspaceRoot);
    case 'reset':
      return resetLocalRuntime(args.slice(1), workspaceRoot);
    case 'status':
    default:
      return continuityStatus(workspaceRoot);
  }
}

module.exports = {
  run,
  continuityStatus,
  initLocalRuntime,
  resetLocalRuntime,
};

if (require.main === module) {
  const result = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  if (!result.ok) process.exit(1);
}
