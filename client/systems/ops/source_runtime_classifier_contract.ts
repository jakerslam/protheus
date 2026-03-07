#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  toBool,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  emit
} = require('../../lib/queued_backlog_runtime');

type AnyObj = Record<string, any>;

const DEFAULT_POLICY_PATH = process.env.SOURCE_RUNTIME_CLASSIFIER_POLICY_PATH
  ? path.resolve(process.env.SOURCE_RUNTIME_CLASSIFIER_POLICY_PATH)
  : (
    fs.existsSync(path.join(ROOT, 'client', 'config', 'source_runtime_classifier_policy.json'))
      ? path.join(ROOT, 'client', 'config', 'source_runtime_classifier_policy.json')
      : path.join(ROOT, 'config', 'source_runtime_classifier_policy.json')
  );

function usage() {
  console.log('Usage:');
  console.log('  node client/systems/ops/source_runtime_classifier_contract.js check [--strict=1|0] [--policy=<path>]');
  console.log('  node client/systems/ops/source_runtime_classifier_contract.js status [--policy=<path>]');
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    runtime_roots: ['client/local', 'core/local'],
    source_memory_root: 'client/memory',
    runtime_memory_roots: ['client/local/memory', 'client/local/state/memory'],
    source_memory_runtime_like_ext: ['.json', '.jsonl', '.sqlite', '.db', '.log', '.lock'],
    source_memory_runtime_like_allow_prefixes: ['tools/tests/'],
    source_memory_runtime_like_allow_files: ['README.md'],
    required_runtime_paths: [
      'client/local/adaptive',
      'client/local/memory',
      'client/local/logs',
      'client/local/secrets',
      'client/local/state',
      'core/local/state',
      'core/local/logs',
      'core/local/cache',
      'core/local/memory'
    ],
    legacy_runtime_root_dirs: [
      'adaptive',
      'memory',
      'habits',
      'logs',
      'secrets',
      'reports',
      'research',
      'patches'
    ],
    forbidden_source_ext_in_runtime: ['.ts', '.tsx', '.js', '.jsx', '.rs', '.c', '.cc', '.cpp', '.h', '.hpp', '.py', '.sh', '.ps1', '.html', '.css'],
    runtime_ignore_files: ['.gitkeep', '.gitignore', 'README.md'],
    runtime_ignore_path_contains: [
      '/local/workspaces/',
      '/security/anti_sabotage/snapshots/',
      '/state/snapshots/',
      '/state/tmp/'
    ],
    paths: {
      latest_path: 'client/local/state/ops/source_runtime_classifier/latest.json',
      receipts_path: 'client/local/state/ops/source_runtime_classifier/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 24) || base.version,
    enabled: toBool(raw.enabled, true),
    strict_default: toBool(raw.strict_default, true),
    runtime_roots: Array.isArray(raw.runtime_roots) ? raw.runtime_roots : base.runtime_roots,
    source_memory_root: cleanText(raw.source_memory_root || base.source_memory_root, 240) || base.source_memory_root,
    runtime_memory_roots: Array.isArray(raw.runtime_memory_roots)
      ? raw.runtime_memory_roots
      : base.runtime_memory_roots,
    source_memory_runtime_like_ext: Array.isArray(raw.source_memory_runtime_like_ext)
      ? raw.source_memory_runtime_like_ext
      : base.source_memory_runtime_like_ext,
    source_memory_runtime_like_allow_prefixes: Array.isArray(raw.source_memory_runtime_like_allow_prefixes)
      ? raw.source_memory_runtime_like_allow_prefixes
      : base.source_memory_runtime_like_allow_prefixes,
    source_memory_runtime_like_allow_files: Array.isArray(raw.source_memory_runtime_like_allow_files)
      ? raw.source_memory_runtime_like_allow_files
      : base.source_memory_runtime_like_allow_files,
    required_runtime_paths: Array.isArray(raw.required_runtime_paths) ? raw.required_runtime_paths : base.required_runtime_paths,
    legacy_runtime_root_dirs: Array.isArray(raw.legacy_runtime_root_dirs)
      ? raw.legacy_runtime_root_dirs
      : base.legacy_runtime_root_dirs,
    forbidden_source_ext_in_runtime: Array.isArray(raw.forbidden_source_ext_in_runtime)
      ? raw.forbidden_source_ext_in_runtime
      : base.forbidden_source_ext_in_runtime,
    runtime_ignore_files: Array.isArray(raw.runtime_ignore_files)
      ? raw.runtime_ignore_files
      : base.runtime_ignore_files,
    runtime_ignore_path_contains: Array.isArray(raw.runtime_ignore_path_contains)
      ? raw.runtime_ignore_path_contains
      : base.runtime_ignore_path_contains,
    paths: {
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    }
  };
}

function walkFiles(absDir: string, out: string[]) {
  if (!fs.existsSync(absDir)) return;
  const rows = fs.readdirSync(absDir, { withFileTypes: true });
  for (const row of rows) {
    const abs = path.join(absDir, row.name);
    if (row.isDirectory()) {
      walkFiles(abs, out);
      continue;
    }
    if (row.isFile()) out.push(abs);
  }
}

function runCheck(policy: AnyObj, strict: boolean) {
  const missingRuntimeRoots = (policy.runtime_roots || [])
    .map((token: unknown) => cleanText(token, 240))
    .filter(Boolean)
    .filter((relPath: string) => !fs.existsSync(path.join(ROOT, relPath)));

  const missingRuntimePaths = (policy.required_runtime_paths || [])
    .map((token: unknown) => cleanText(token, 240))
    .filter(Boolean)
    .filter((relPath: string) => !fs.existsSync(path.join(ROOT, relPath)));

  const legacyRuntimeRootsPresent = (policy.legacy_runtime_root_dirs || [])
    .map((token: unknown) => cleanText(token, 120))
    .filter(Boolean)
    .filter((name: string) => fs.existsSync(path.join(ROOT, name)));

  const forbiddenExt = new Set(
    (policy.forbidden_source_ext_in_runtime || [])
      .map((v: unknown) => String(v || '').trim())
      .filter(Boolean)
  );
  const ignoreFiles = new Set(
    (policy.runtime_ignore_files || [])
      .map((v: unknown) => String(v || '').trim())
      .filter(Boolean)
  );
  const ignorePathContains = (policy.runtime_ignore_path_contains || [])
    .map((v: unknown) => cleanText(v, 260))
    .filter(Boolean);

  const runtimeSourceViolations: AnyObj[] = [];
  let runtimeFilesScanned = 0;

  for (const relRoot of (policy.runtime_roots || [])) {
    const cleanRoot = cleanText(relRoot, 240);
    if (!cleanRoot) continue;
    const absRoot = path.join(ROOT, cleanRoot);
    const files: string[] = [];
    walkFiles(absRoot, files);
    runtimeFilesScanned += files.length;

    for (const absFile of files) {
      const fileName = path.basename(absFile);
      if (ignoreFiles.has(fileName)) continue;
      const relFile = rel(absFile);
      if (ignorePathContains.some((needle: string) => relFile.includes(needle))) continue;
      const ext = path.extname(fileName);
      if (!forbiddenExt.has(ext)) continue;
      runtimeSourceViolations.push({
        file: relFile,
        ext
      });
    }
  }

  const sourceMemoryRoot = cleanText(policy.source_memory_root, 240);
  const runtimeMemoryRootsMissing = (policy.runtime_memory_roots || [])
    .map((token: unknown) => cleanText(token, 240))
    .filter(Boolean)
    .filter((relPath: string) => !fs.existsSync(path.join(ROOT, relPath)));
  const sourceMemoryRuntimeLikeViolations: AnyObj[] = [];
  let sourceMemoryFilesScanned = 0;

  if (sourceMemoryRoot) {
    const absSourceMemoryRoot = path.join(ROOT, sourceMemoryRoot);
    const sourceMemoryExists = fs.existsSync(absSourceMemoryRoot);
    if (sourceMemoryExists) {
      const sourceMemoryFiles: string[] = [];
      walkFiles(absSourceMemoryRoot, sourceMemoryFiles);
      sourceMemoryFilesScanned = sourceMemoryFiles.length;
      const runtimeLikeExt = new Set(
        (policy.source_memory_runtime_like_ext || [])
          .map((v: unknown) => String(v || '').trim().toLowerCase())
          .filter(Boolean)
      );
      const allowPrefixes = (policy.source_memory_runtime_like_allow_prefixes || [])
        .map((v: unknown) => cleanText(v, 260))
        .filter(Boolean);
      const allowFiles = new Set(
        (policy.source_memory_runtime_like_allow_files || [])
          .map((v: unknown) => cleanText(v, 260))
          .filter(Boolean)
      );
      for (const absFile of sourceMemoryFiles) {
        const relFile = rel(absFile);
        const relWithinSource = relFile.startsWith(`${sourceMemoryRoot}/`)
          ? relFile.slice(sourceMemoryRoot.length + 1)
          : relFile;
        const fileName = path.basename(relFile);
        if (allowFiles.has(relWithinSource) || allowFiles.has(fileName)) continue;
        if (allowPrefixes.some((prefix: string) => relWithinSource.startsWith(prefix))) continue;
        const ext = path.extname(fileName).toLowerCase();
        if (!runtimeLikeExt.has(ext)) continue;
        sourceMemoryRuntimeLikeViolations.push({
          file: relFile,
          ext
        });
      }
    }
  }

  const checks = {
    runtime_roots_exist: missingRuntimeRoots.length === 0,
    required_runtime_paths_present: missingRuntimePaths.length === 0,
    runtime_memory_roots_present: runtimeMemoryRootsMissing.length === 0,
    no_legacy_runtime_roots: legacyRuntimeRootsPresent.length === 0,
    no_source_files_in_runtime_roots: runtimeSourceViolations.length === 0,
    no_runtime_like_files_in_source_memory: sourceMemoryRuntimeLikeViolations.length === 0
  };

  const blocking = Object.entries(checks)
    .filter(([, ok]) => ok !== true)
    .map(([k]) => k);

  const pass = blocking.length === 0;
  const ok = strict ? pass : true;

  const out = {
    ok,
    pass,
    strict,
    type: 'source_runtime_classifier_contract',
    ts: nowIso(),
    checks,
    blocking_checks: blocking,
    counts: {
      runtime_files_scanned: runtimeFilesScanned,
      source_memory_files_scanned: sourceMemoryFilesScanned,
      runtime_source_violations: runtimeSourceViolations.length,
      source_memory_runtime_like_violations: sourceMemoryRuntimeLikeViolations.length,
      missing_runtime_roots: missingRuntimeRoots.length,
      missing_runtime_paths: missingRuntimePaths.length,
      runtime_memory_roots_missing: runtimeMemoryRootsMissing.length,
      legacy_runtime_roots_present: legacyRuntimeRootsPresent.length
    },
    missing_runtime_roots: missingRuntimeRoots,
    missing_runtime_paths: missingRuntimePaths,
    runtime_memory_roots_missing: runtimeMemoryRootsMissing,
    legacy_runtime_roots_present: legacyRuntimeRootsPresent,
    runtime_source_violations: runtimeSourceViolations.slice(0, 500),
    source_memory_runtime_like_violations: sourceMemoryRuntimeLikeViolations.slice(0, 500)
  };

  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  return out;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'check').toLowerCase();
  if (args.help || cmd === 'help' || cmd === '--help') {
    usage();
    return emit({ ok: true, help: true }, 0);
  }

  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);

  if (cmd === 'status') {
    return emit(readJson(policy.paths.latest_path, {
      ok: true,
      type: 'source_runtime_classifier_contract',
      status: 'no_status'
    }), 0);
  }

  if (cmd !== 'check') {
    usage();
    return emit({ ok: false, error: `unknown_command:${cmd}` }, 1);
  }

  const strict = toBool(args.strict, policy.strict_default);
  const out = runCheck(policy, strict);
  return emit(out, out.ok ? 0 : 1);
}

main();
