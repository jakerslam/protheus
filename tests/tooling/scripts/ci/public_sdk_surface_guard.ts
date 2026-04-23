#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const REQUIRED_REFERENCE_APPS = [
  'examples/apps/reference-task-submit/src/main.ts',
  'examples/apps/reference-receipts-memory/src/main.ts',
  'examples/apps/reference-assimilation-policy/src/main.ts',
];

function parseFlag(argv: string[], key: string, fallback = ''): string {
  const prefix = `--${key}=`;
  for (const arg of argv) {
    const raw = String(arg || '').trim();
    if (raw.startsWith(prefix)) {
      return raw.slice(prefix.length);
    }
  }
  return fallback;
}

function hasFlag(argv: string[], key: string): boolean {
  const exact = `--${key}`;
  return argv.some((arg) => String(arg || '').trim() === exact);
}

function readText(filePath: string): string {
  return fs.readFileSync(filePath, 'utf8');
}

function fileExists(filePath: string): boolean {
  try {
    return fs.statSync(filePath).isFile();
  } catch {
    return false;
  }
}

function duplicateValues(values: string[]): string[] {
  const counts = new Map<string, number>();
  for (const value of values) counts.set(value, (counts.get(value) || 0) + 1);
  return [...counts.entries()]
    .filter(([, count]) => count > 1)
    .map(([value]) => value)
    .sort();
}

function isCanonicalRelativePath(token: string): boolean {
  const value = String(token || '');
  if (!value) return false;
  if (value.trim() !== value) return false;
  if (value.includes('\\')) return false;
  if (value.startsWith('/') || value.startsWith('./') || value.startsWith('../')) return false;
  if (value.includes('//')) return false;
  const segments = value.split('/');
  if (segments.some((segment) => !segment || segment === '.' || segment === '..')) return false;
  return true;
}

function countRegexMatches(source: string, regex: RegExp): number {
  const flags = regex.flags.includes('g') ? regex.flags : `${regex.flags}g`;
  const probe = new RegExp(regex.source, flags);
  return (source.match(probe) || []).length;
}

function buildReferenceApps(files: string[]): {
  ok: boolean;
  artifact_dir: string;
  esbuild_cli: string;
  runs: Array<{ path: string; ok: boolean; status: number; stderr: string; stdout: string; out_file: string }>;
} {
  const artifactDir = path.join(ROOT, 'core', 'local', 'artifacts', 'public_sdk_reference_build');
  fs.mkdirSync(artifactDir, { recursive: true });
  const esbuildCli = path.join(ROOT, 'node_modules', '.bin', 'esbuild');

  if (!fileExists(esbuildCli)) {
    return {
      ok: false,
      artifact_dir: artifactDir,
      esbuild_cli: esbuildCli,
      runs: files.map((relPath) => ({
        path: relPath,
        ok: false,
        status: -1,
        stderr: 'esbuild_cli_missing',
        stdout: '',
        out_file: '',
      })),
    };
  }

  const runs = files.map((relPath) => {
    const outFile = path.join(
      artifactDir,
      relPath.replace(/\//g, '__').replace(/\.ts$/, '.cjs')
    );
    const proc = spawnSync(
      esbuildCli,
      [
        relPath,
        '--bundle',
        '--platform=node',
        '--format=cjs',
        '--target=node20',
        `--outfile=${outFile}`,
        '--alias:@infring/sdk=./packages/infring-sdk/src/index.ts',
      ],
      {
        cwd: ROOT,
        encoding: 'utf8',
        maxBuffer: 16 * 1024 * 1024,
      }
    );
    return {
      path: relPath,
      ok: Number(proc.status || 0) === 0,
      status: Number(proc.status || 0),
      stderr: String(proc.stderr || ''),
      stdout: String(proc.stdout || ''),
      out_file: outFile,
    };
  });
  return {
    ok: runs.every((row) => row.ok),
    artifact_dir: artifactDir,
    esbuild_cli: esbuildCli,
    runs,
  };
}

function run(argv: string[]): number {
  const strict = hasFlag(argv, 'strict') || parseFlag(argv, 'strict', '0') === '1';
  const expectedFiles = [...REQUIRED_REFERENCE_APPS];
  const sdkImportRegex = /from\s+['"]@infring\/sdk['"]/;
  const sdkExactImportRegex = /from\s+['"]@infring\/sdk['"]/g;
  const sdkSubpathImportRegex = /from\s+['"]@infring\/sdk\//;
  const forbiddenImportRegex = /from\s+['"](\.\.\/\.\.\/|client\/|core\/|\/Users\/)/;
  const forbiddenRequireRegex = /require\(\s*['"](\.\.\/\.\.\/|client\/|core\/|\/Users\/)/;
  const policyFailures: Array<{ reason: string; markers?: string[] }> = [];

  const fileChecks = expectedFiles.map((relPath) => {
    const absPath = path.join(ROOT, relPath);
    const exists = fileExists(absPath);
    if (!exists) {
      return {
        path: relPath,
        exists: false,
        sdk_import_only: false,
        sdk_import_count: 0,
        sdk_subpath_import: false,
        forbidden_internal_import: true,
        forbidden_internal_require: false,
      };
    }
    const source = readText(absPath);
    return {
      path: relPath,
      exists: true,
      sdk_import_only: sdkImportRegex.test(source),
      sdk_import_count: countRegexMatches(source, sdkExactImportRegex),
      sdk_subpath_import: sdkSubpathImportRegex.test(source),
      forbidden_internal_import: forbiddenImportRegex.test(source),
      forbidden_internal_require: forbiddenRequireRegex.test(source),
    };
  });

  if (expectedFiles.length === 0) {
    policyFailures.push({ reason: 'expected_reference_files_empty' });
  }
  const duplicateExpectedFiles = duplicateValues(expectedFiles);
  if (duplicateExpectedFiles.length > 0) {
    policyFailures.push({
      reason: 'expected_reference_files_duplicate',
      markers: duplicateExpectedFiles,
    });
  }
  const nonCanonicalExpectedFiles = expectedFiles.filter((row) => !isCanonicalRelativePath(row));
  if (nonCanonicalExpectedFiles.length > 0) {
    policyFailures.push({
      reason: 'expected_reference_files_noncanonical',
      markers: nonCanonicalExpectedFiles.sort(),
    });
  }
  const nonTsExpectedFiles = expectedFiles.filter((row) => !row.endsWith('.ts'));
  if (nonTsExpectedFiles.length > 0) {
    policyFailures.push({
      reason: 'expected_reference_files_non_ts',
      markers: nonTsExpectedFiles.sort(),
    });
  }
  const unsortedExpected = [...expectedFiles].sort((a, b) => a.localeCompare(b));
  if (unsortedExpected.join('|') !== expectedFiles.join('|')) {
    policyFailures.push({
      reason: 'expected_reference_files_order_drift',
      markers: expectedFiles,
    });
  }
  const missingBaselineExpected = REQUIRED_REFERENCE_APPS.filter((row) => !expectedFiles.includes(row));
  if (missingBaselineExpected.length > 0) {
    policyFailures.push({
      reason: 'required_reference_files_missing',
      markers: missingBaselineExpected.sort(),
    });
  }
  if (!forbiddenImportRegex.test("from 'core/layer2/nexus/src/lib.rs'")) {
    policyFailures.push({
      reason: 'forbidden_import_regex_contract_drift_core',
    });
  }
  if (!forbiddenImportRegex.test("from 'client/runtime/systems/ops'")) {
    policyFailures.push({
      reason: 'forbidden_import_regex_contract_drift_client',
    });
  }
  if (!forbiddenImportRegex.test("from '/Users/example/private'")) {
    policyFailures.push({
      reason: 'forbidden_import_regex_contract_drift_absolute',
    });
  }
  if (!forbiddenRequireRegex.test("require('core/layer2/nexus/src/lib.rs')")) {
    policyFailures.push({
      reason: 'forbidden_require_regex_contract_drift_core',
    });
  }
  if (!sdkImportRegex.test("import { createClient } from '@infring/sdk'")) {
    policyFailures.push({
      reason: 'sdk_import_regex_contract_drift',
    });
  }

  const compile = buildReferenceApps(expectedFiles);
  if (!fileExists(compile.esbuild_cli)) {
    policyFailures.push({
      reason: 'esbuild_cli_missing',
      markers: [compile.esbuild_cli],
    });
  }
  if (!isCanonicalRelativePath(path.relative(ROOT, compile.artifact_dir).replace(/\\/g, '/'))) {
    policyFailures.push({
      reason: 'compile_artifact_dir_noncanonical',
      markers: [compile.artifact_dir],
    });
  }
  if (!compile.artifact_dir.startsWith(path.join(ROOT, 'core', 'local', 'artifacts'))) {
    policyFailures.push({
      reason: 'compile_artifact_dir_outside_artifacts_root',
      markers: [compile.artifact_dir],
    });
  }
  const compileRunPaths = compile.runs.map((row) => String(row.path || ''));
  if (compile.runs.length !== expectedFiles.length) {
    policyFailures.push({
      reason: 'compile_run_count_drift',
      markers: [String(compile.runs.length), String(expectedFiles.length)],
    });
  }
  const duplicateCompileRunPaths = duplicateValues(compileRunPaths);
  if (duplicateCompileRunPaths.length > 0) {
    policyFailures.push({
      reason: 'compile_run_paths_duplicate',
      markers: duplicateCompileRunPaths,
    });
  }
  const unknownCompileRunPaths = compileRunPaths.filter((row) => !expectedFiles.includes(row));
  if (unknownCompileRunPaths.length > 0) {
    policyFailures.push({
      reason: 'compile_run_paths_unknown',
      markers: unknownCompileRunPaths.sort(),
    });
  }
  const missingCompileRunPaths = expectedFiles.filter((row) => !compileRunPaths.includes(row));
  if (missingCompileRunPaths.length > 0) {
    policyFailures.push({
      reason: 'compile_run_paths_missing',
      markers: missingCompileRunPaths.sort(),
    });
  }
  for (const row of compile.runs) {
    if (!Number.isInteger(row.status)) {
      policyFailures.push({
        reason: 'compile_status_non_integer',
        markers: [row.path, String(row.status)],
      });
    }
    if (row.ok !== (row.status === 0)) {
      policyFailures.push({
        reason: 'compile_row_ok_status_drift',
        markers: [row.path, String(row.ok), String(row.status)],
      });
    }
    const outFile = String(row.out_file || '');
    if (row.ok && !fileExists(outFile)) {
      policyFailures.push({
        reason: 'compile_row_output_missing',
        markers: [row.path, outFile],
      });
    }
    if (outFile && !outFile.startsWith(compile.artifact_dir)) {
      policyFailures.push({
        reason: 'compile_row_output_outside_artifact_dir',
        markers: [row.path, outFile],
      });
    }
    if (outFile && !outFile.endsWith('.cjs')) {
      policyFailures.push({
        reason: 'compile_row_output_non_cjs_suffix',
        markers: [row.path, outFile],
      });
    }
  }
  if (compile.ok !== compile.runs.every((row) => row.ok)) {
    policyFailures.push({
      reason: 'compile_aggregate_ok_drift',
    });
  }

  const fileCheckPaths = fileChecks.map((row) => String(row.path || ''));
  if (fileChecks.length !== expectedFiles.length) {
    policyFailures.push({
      reason: 'file_check_count_drift',
      markers: [String(fileChecks.length), String(expectedFiles.length)],
    });
  }
  const duplicateFileCheckPaths = duplicateValues(fileCheckPaths);
  if (duplicateFileCheckPaths.length > 0) {
    policyFailures.push({
      reason: 'file_check_paths_duplicate',
      markers: duplicateFileCheckPaths,
    });
  }
  const missingFileCheckPaths = expectedFiles.filter((row) => !fileCheckPaths.includes(row));
  if (missingFileCheckPaths.length > 0) {
    policyFailures.push({
      reason: 'file_check_paths_missing',
      markers: missingFileCheckPaths.sort(),
    });
  }
  const unknownFileCheckPaths = fileCheckPaths.filter((row) => !expectedFiles.includes(row));
  if (unknownFileCheckPaths.length > 0) {
    policyFailures.push({
      reason: 'file_check_paths_unknown',
      markers: unknownFileCheckPaths.sort(),
    });
  }

  const violations: Array<{ path: string; reason: string; markers?: string[] }> = [];
  for (const row of fileChecks) {
    if (!row.exists) {
      violations.push({ path: row.path, reason: 'reference_file_missing' });
      continue;
    }
    if (!row.sdk_import_only) {
      violations.push({ path: row.path, reason: 'sdk_import_missing' });
    }
    if (row.sdk_import_count !== 1) {
      violations.push({
        path: row.path,
        reason: 'sdk_import_count_invalid',
        markers: [String(row.sdk_import_count)],
      });
    }
    if (row.sdk_subpath_import) {
      violations.push({ path: row.path, reason: 'sdk_subpath_import_forbidden' });
    }
    if (row.forbidden_internal_import) {
      violations.push({ path: row.path, reason: 'internal_import_forbidden' });
    }
    if (row.forbidden_internal_require) {
      violations.push({ path: row.path, reason: 'internal_require_forbidden' });
    }
  }
  for (const row of compile.runs) {
    if (!row.ok) {
      violations.push({
        path: row.path,
        reason: 'reference_compile_failed',
        markers: [String(row.status)],
      });
    }
  }

  const allFilesPass = fileChecks.every(
    (row) =>
      row.exists &&
      row.sdk_import_only &&
      row.sdk_import_count === 1 &&
      !row.sdk_subpath_import &&
      !row.forbidden_internal_import &&
      !row.forbidden_internal_require
  );

  const out = {
    ok: policyFailures.length === 0 && violations.length === 0 && allFilesPass && compile.ok,
    type: 'public_sdk_surface_guard',
    build_engine: 'esbuild',
    summary: {
      expected_reference_app_count: expectedFiles.length,
      policy_failure_count: policyFailures.length,
      violation_count: violations.length,
      compile_failure_count: compile.runs.filter((row) => !row.ok).length,
    },
    policy_failures: policyFailures,
    violations,
    checks: {
      expected_reference_app_count: expectedFiles.length,
      file_checks: fileChecks,
      compile,
    },
  };

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);

  if (!out.ok && strict) {
    return 1;
  }
  return out.ok ? 0 : 2;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
