#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/release_proof_pack_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/release_proof_pack_manifest.json',
      400,
    ),
    version: cleanText(readFlag(argv, 'version') || new Date().toISOString().slice(0, 10), 120),
  };
}

function ensureParent(absPath: string) {
  const parent = path.dirname(absPath);
  fs.mkdirSync(parent, { recursive: true });
}

function copyIntoPack(root: string, relPath: string, packRoot: string) {
  const source = path.resolve(root, relPath);
  const exists = fs.existsSync(source);
  const destination = path.resolve(packRoot, relPath);
  if (exists) {
    ensureParent(destination);
    fs.copyFileSync(source, destination);
  }
  return { path: relPath, exists, source, destination };
}

function markdown(report: any): string {
  const lines = [
    '# Release Proof Pack',
    '',
    `- version: ${report.version}`,
    `- pack_root: ${report.pack_root}`,
    `- required_missing: ${report.summary.required_missing}`,
    '',
    '| artifact | required | exists |',
    '| --- | :---: | :---: |',
  ];
  for (const row of report.artifacts) {
    lines.push(`| ${row.path} | ${row.required ? 'yes' : 'no'} | ${row.exists ? 'yes' : 'no'} |`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestRaw = fs.readFileSync(path.resolve(root, args.manifestPath), 'utf8');
  const manifest = JSON.parse(manifestRaw) as {
    required_artifacts: string[];
    optional_artifacts: string[];
  };

  const packRoot = path.resolve(root, 'releases', 'proof-packs', args.version);
  fs.mkdirSync(packRoot, { recursive: true });

  const artifactRows: Array<{ path: string; required: boolean; exists: boolean; source: string; destination: string }> = [];

  for (const rel of manifest.required_artifacts || []) {
    artifactRows.push({ ...copyIntoPack(root, rel, packRoot), required: true });
  }
  for (const rel of manifest.optional_artifacts || []) {
    artifactRows.push({ ...copyIntoPack(root, rel, packRoot), required: false });
  }

  const requiredMissing = artifactRows.filter((row) => row.required && !row.exists).map((row) => row.path);

  const packManifest = {
    ok: requiredMissing.length === 0,
    type: 'release_proof_pack_manifest',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    artifacts: artifactRows,
    required_missing: requiredMissing,
  };

  const packManifestPath = path.resolve(packRoot, 'manifest.json');
  ensureParent(packManifestPath);
  fs.writeFileSync(packManifestPath, `${JSON.stringify(packManifest, null, 2)}\n`, 'utf8');

  const reportPath = path.resolve(packRoot, 'README.md');
  writeTextArtifact(reportPath, markdown({ ...packManifest, summary: { required_missing: requiredMissing.length } }));

  const report = {
    ok: requiredMissing.length === 0,
    type: 'release_proof_pack_assemble',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    version: args.version,
    pack_root: packRoot,
    source_manifest_path: args.manifestPath,
    summary: {
      artifact_count: artifactRows.length,
      required_missing: requiredMissing.length,
      pass: requiredMissing.length === 0,
    },
    artifacts: artifactRows,
    failures: requiredMissing.map((detail) => ({ id: 'proof_pack_required_artifact_missing', detail })),
    artifact_paths: [packManifestPath, reportPath],
  };

  return emitStructuredResult(report, {
    outPath: args.outPath,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
