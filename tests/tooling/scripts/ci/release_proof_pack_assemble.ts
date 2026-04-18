#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type PackManifest = {
  version: number;
  artifact_groups?: Record<string, string[]>;
  required_artifacts: string[];
  optional_artifacts: string[];
};

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

function sha256File(absPath: string): string {
  const data = fs.readFileSync(absPath);
  return createHash('sha256').update(data).digest('hex');
}

function categoryLookup(manifest: PackManifest): Map<string, string> {
  const out = new Map<string, string>();
  const groups = manifest.artifact_groups || {};
  for (const [group, rows] of Object.entries(groups)) {
    for (const relPath of rows || []) {
      out.set(cleanText(relPath, 400), cleanText(group, 120));
    }
  }
  return out;
}

function copyIntoPack(root: string, relPath: string, packRoot: string) {
  const source = path.resolve(root, relPath);
  const exists = fs.existsSync(source);
  const destination = path.resolve(packRoot, relPath);
  let checksum = '';
  let sizeBytes = 0;
  if (exists) {
    ensureParent(destination);
    fs.copyFileSync(source, destination);
    checksum = sha256File(destination);
    sizeBytes = fs.statSync(destination).size;
  }
  return { path: relPath, exists, source, destination, checksum, size_bytes: sizeBytes };
}

function markdown(report: any): string {
  const lines = [
    '# Release Proof Pack',
    '',
    `- version: ${report.version}`,
    `- pack_root: ${report.pack_root}`,
    `- required_missing: ${report.summary.required_missing}`,
    '',
    '| artifact | category | required | exists | sha256 |',
    '| --- | --- | :---: | :---: | --- |',
  ];
  for (const row of report.artifacts) {
    lines.push(
      `| ${row.path} | ${row.category} | ${row.required ? 'yes' : 'no'} | ${row.exists ? 'yes' : 'no'} | ${
        row.exists ? row.checksum : 'missing'
      } |`,
    );
  }
  lines.push('');
  lines.push('## Category summary');
  for (const group of report.category_summary) {
    lines.push(
      `- ${group.category}: present=${group.present}/${group.total};required_missing=${group.required_missing}`,
    );
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  const manifestRaw = fs.readFileSync(path.resolve(root, args.manifestPath), 'utf8');
  const manifest = JSON.parse(manifestRaw) as PackManifest;
  const categoryByPath = categoryLookup(manifest);

  const packRoot = path.resolve(root, 'releases', 'proof-packs', args.version);
  fs.mkdirSync(packRoot, { recursive: true });

  const artifactRows: Array<{
    path: string;
    category: string;
    required: boolean;
    exists: boolean;
    source: string;
    destination: string;
    checksum: string;
    size_bytes: number;
  }> = [];

  for (const rel of manifest.required_artifacts || []) {
    const normalized = cleanText(rel, 400);
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: true,
    });
  }
  for (const rel of manifest.optional_artifacts || []) {
    const normalized = cleanText(rel, 400);
    artifactRows.push({
      ...copyIntoPack(root, normalized, packRoot),
      category: cleanText(categoryByPath.get(normalized) || 'ungrouped', 120),
      required: false,
    });
  }

  const requiredMissing = artifactRows.filter((row) => row.required && !row.exists).map((row) => row.path);
  const categories = Array.from(new Set(artifactRows.map((row) => row.category)));
  const categorySummary = categories.map((category) => {
    const rows = artifactRows.filter((row) => row.category === category);
    const present = rows.filter((row) => row.exists).length;
    const requiredMissingCount = rows.filter((row) => row.required && !row.exists).length;
    return {
      category,
      total: rows.length,
      present,
      required_missing: requiredMissingCount,
    };
  });

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
    category_summary: categorySummary,
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
    category_summary: categorySummary,
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
