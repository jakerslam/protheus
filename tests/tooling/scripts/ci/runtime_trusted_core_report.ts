#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type TrustedCoreManifest = {
  version: number;
  trusted_modules: string[];
  non_authoritative_surfaces: string[];
  bridge_points: Array<{ id: string; from: string; to: string; policy_choke_point: string }>;
  policy_choke_points: string[];
  fallback_declarations: Array<{ id: string; scope: string; mode: string; fail_closed: boolean }>;
};

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_trusted_core_report_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    manifestPath: cleanText(
      readFlag(argv, 'manifest') || 'tests/tooling/config/trusted_core_manifest.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/RUNTIME_TRUSTED_CORE_REPORT_CURRENT.md',
      400,
    ),
  };
}

function markdown(report: any): string {
  const lines = [
    '# Runtime Trusted Core Report',
    '',
    `- manifest: ${report.manifest_path}`,
    `- trusted_module_count: ${report.summary.trusted_module_count}`,
    `- bridge_count: ${report.summary.bridge_count}`,
    `- drift_count: ${report.summary.drift_count}`,
    '',
    '## Trusted Modules',
  ];
  for (const row of report.trusted_modules) {
    lines.push(`- ${row.path} (${row.exists ? 'present' : 'missing'})`);
  }
  lines.push('');
  lines.push('## Bridge Points');
  for (const row of report.bridge_points) {
    lines.push(`- ${row.id}: ${row.from} -> ${row.to} (choke=${row.policy_choke_point})`);
  }
  lines.push('');
  lines.push('## Fallback Declarations');
  for (const row of report.fallback_declarations) {
    lines.push(`- ${row.id}: scope=${row.scope}, mode=${row.mode}, fail_closed=${row.fail_closed}`);
  }
  lines.push('');
  lines.push('## Drift');
  if (report.drift.length === 0) {
    lines.push('- none');
  } else {
    for (const row of report.drift) lines.push(`- ${row}`);
  }
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);

  let manifest: TrustedCoreManifest;
  try {
    const raw = fs.readFileSync(path.resolve(root, args.manifestPath), 'utf8');
    manifest = JSON.parse(raw) as TrustedCoreManifest;
  } catch (err) {
    const payload = {
      ok: false,
      type: 'runtime_trusted_core_report',
      error: 'trusted_core_manifest_read_failed',
      detail: cleanText(String(err), 400),
      manifest_path: args.manifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const drift: string[] = [];

  const trustedModules = (manifest.trusted_modules || []).map((row) => {
    const rel = cleanText(row, 500);
    const abs = path.resolve(root, rel);
    const exists = fs.existsSync(abs);
    if (!exists) drift.push(`trusted_module_missing:${rel}`);
    return { path: rel, exists };
  });

  const duplicateTrusted = new Set<string>();
  const seenTrusted = new Set<string>();
  for (const row of manifest.trusted_modules || []) {
    if (seenTrusted.has(row)) duplicateTrusted.add(row);
    seenTrusted.add(row);
  }
  for (const row of duplicateTrusted) drift.push(`trusted_module_duplicate:${row}`);

  const chokePoints = new Set((manifest.policy_choke_points || []).map((row) => cleanText(row, 200)));
  for (const row of manifest.bridge_points || []) {
    if (!chokePoints.has(cleanText(row.policy_choke_point || '', 200))) {
      drift.push(`bridge_missing_policy_choke:${row.id}`);
    }
  }

  for (const row of manifest.fallback_declarations || []) {
    if (!row.fail_closed) {
      drift.push(`fallback_not_fail_closed:${row.id}`);
    }
  }

  const report = {
    ok: drift.length === 0,
    type: 'runtime_trusted_core_report',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    manifest_path: args.manifestPath,
    markdown_path: args.markdownOutPath,
    summary: {
      trusted_module_count: trustedModules.length,
      bridge_count: (manifest.bridge_points || []).length,
      fallback_count: (manifest.fallback_declarations || []).length,
      drift_count: drift.length,
      pass: drift.length === 0,
    },
    trusted_modules: trustedModules,
    non_authoritative_surfaces: manifest.non_authoritative_surfaces || [],
    bridge_points: manifest.bridge_points || [],
    policy_choke_points: manifest.policy_choke_points || [],
    fallback_declarations: manifest.fallback_declarations || [],
    drift,
    failures: drift.map((detail) => ({ id: 'trusted_core_drift', detail })),
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, markdown(report));

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
