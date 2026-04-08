#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');

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

function buildReferenceApps(files: string[]): {
  ok: boolean;
  runs: Array<{ path: string; ok: boolean; status: number; stderr: string; stdout: string }>;
} {
  const artifactDir = path.join(ROOT, 'core', 'local', 'artifacts', 'public_sdk_reference_build');
  fs.mkdirSync(artifactDir, { recursive: true });
  const esbuildCli = path.join(ROOT, 'node_modules', '.bin', 'esbuild');

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
    };
  });
  return {
    ok: runs.every((row) => row.ok),
    runs,
  };
}

function run(argv: string[]): number {
  const strict = hasFlag(argv, 'strict') || parseFlag(argv, 'strict', '0') === '1';
  const expectedFiles = [
    'examples/apps/reference-task-submit/src/main.ts',
    'examples/apps/reference-receipts-memory/src/main.ts',
    'examples/apps/reference-assimilation-policy/src/main.ts',
  ];
  const sdkImportRegex = /from\s+['"]@infring\/sdk['"]/;
  const forbiddenImportRegex = /from\s+['"](\.\.\/\.\.\/|client\/|core\/|\/Users\/)/;

  const fileChecks = expectedFiles.map((relPath) => {
    const absPath = path.join(ROOT, relPath);
    const exists = fileExists(absPath);
    if (!exists) {
      return {
        path: relPath,
        exists: false,
        sdk_import_only: false,
        forbidden_internal_import: true,
      };
    }
    const source = readText(absPath);
    return {
      path: relPath,
      exists: true,
      sdk_import_only: sdkImportRegex.test(source),
      forbidden_internal_import: forbiddenImportRegex.test(source),
    };
  });

  const compile = buildReferenceApps(expectedFiles);
  const allFilesPass = fileChecks.every(
    (row) => row.exists && row.sdk_import_only && !row.forbidden_internal_import
  );

  const out = {
    ok: allFilesPass && compile.ok,
    type: 'public_sdk_surface_guard',
    build_engine: 'esbuild',
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
