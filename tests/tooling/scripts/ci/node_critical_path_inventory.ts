#!/usr/bin/env tsx

import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeTextArtifact } from '../../lib/result.ts';

type ScriptClass = 'rust_native' | 'node_typescript' | 'npm_wrapper' | 'unknown';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/node_critical_path_inventory_current.json',
  });
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    baselinePath: cleanText(
      readFlag(argv, 'baseline') || 'core/local/artifacts/node_critical_path_inventory_baseline.json',
      400,
    ),
    markdownOutPath: cleanText(
      readFlag(argv, 'out-markdown') || 'local/workspace/reports/NODE_CRITICAL_PATH_INVENTORY_CURRENT.md',
      400,
    ),
    packagePath: cleanText(readFlag(argv, 'package') || 'package.json', 400),
  };
}

function readJsonBestEffort(filePath: string): { ok: boolean; payload: any; detail: string } {
  try {
    return {
      ok: true,
      payload: JSON.parse(fs.readFileSync(filePath, 'utf8')),
      detail: 'loaded',
    };
  } catch (error) {
    return {
      ok: false,
      payload: null,
      detail: cleanText((error as Error)?.message || 'json_unavailable', 240),
    };
  }
}

function classifyScriptCommand(command: string): ScriptClass {
  const normalized = cleanText(command, 2000).toLowerCase();
  if (!normalized) return 'unknown';
  if (normalized.includes('cargo run') || normalized.includes('cargo test')) return 'rust_native';
  if (normalized.includes('node client/runtime/lib/ts_entrypoint.ts')) return 'node_typescript';
  if (normalized.startsWith('npm run -s ')) return 'npm_wrapper';
  return 'unknown';
}

function markdown(payload: any): string {
  const lines = [
    '# Node Critical Path Inventory',
    '',
    `Generated: ${payload.generated_at}`,
    `Revision: ${payload.revision}`,
    `Pass: ${payload.ok}`,
    '',
    '## Summary',
    `- critical_scripts_total: ${payload.summary.critical_scripts_total}`,
    `- critical_scripts_missing: ${payload.summary.critical_scripts_missing}`,
    `- rust_native: ${payload.summary.rust_native_count}`,
    `- node_typescript: ${payload.summary.node_typescript_count}`,
    `- npm_wrapper: ${payload.summary.npm_wrapper_count}`,
    `- unknown: ${payload.summary.unknown_count}`,
    `- node_dependency_ratio: ${payload.summary.node_dependency_ratio}`,
    '',
    '| script | class | command |',
    '| --- | --- | --- |',
  ];
  for (const row of payload.rows) {
    lines.push(`| ${row.id} | ${row.classification} | ${row.command} |`);
  }
  lines.push('');
  lines.push('## Failures');
  if (!payload.failures.length) lines.push('- none');
  else payload.failures.forEach((row: any) => lines.push(`- ${row.id}: ${row.detail}`));
  lines.push('');
  return `${lines.join('\n')}\n`;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  const packagePath = path.resolve(root, args.packagePath);
  const packageJson = readJsonBestEffort(packagePath);
  if (!packageJson.ok) {
    const payload = {
      ok: false,
      type: 'node_critical_path_inventory',
      error: 'package_json_unavailable',
      detail: packageJson.detail,
      package_path: args.packagePath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const scripts = packageJson.payload?.scripts || {};
  const criticalScriptIds = [
    'ops:runtime-proof:verify',
    'ops:adapter-runtime-chaos:gate',
    'ops:layer2:parity:guard',
    'ops:trusted-core:report',
    'ops:release:proof-pack',
    'ops:release:scorecard:gate',
    'ops:production-closure:gate',
    'ops:release:verdict',
    'ops:stateful-upgrade-rollback:gate',
    'ops:support-bundle:export',
  ];

  const rows = criticalScriptIds.map((id) => {
    const command = cleanText(scripts?.[id] || '', 2000);
    const classification = classifyScriptCommand(command);
    return {
      id,
      command,
      classification,
      exists: command.length > 0,
    };
  });

  const missing = rows.filter((row) => !row.exists);
  const rustNativeCount = rows.filter((row) => row.classification === 'rust_native').length;
  const nodeTypescriptCount = rows.filter((row) => row.classification === 'node_typescript').length;
  const npmWrapperCount = rows.filter((row) => row.classification === 'npm_wrapper').length;
  const unknownCount = rows.filter((row) => row.classification === 'unknown').length;
  const nodeDependentCount = nodeTypescriptCount + npmWrapperCount;
  const total = rows.length || 1;
  const nodeDependencyRatio = Number((nodeDependentCount / total).toFixed(4));

  const baseline = readJsonBestEffort(path.resolve(root, args.baselinePath));
  const baselineNodeRatio = Number(baseline.payload?.summary?.node_dependency_ratio ?? Number.NaN);
  const baselineAvailable = baseline.ok;
  const nodeDependencyRegression =
    baselineAvailable && Number.isFinite(baselineNodeRatio) && nodeDependencyRatio > baselineNodeRatio;

  const failures = []
    .concat(
      missing.map((row) => ({
        id: 'critical_script_missing',
        detail: row.id,
      })),
    )
    .concat(
      nodeDependencyRegression
        ? [
            {
              id: 'node_dependency_ratio_regression',
              detail: `current=${nodeDependencyRatio};baseline=${baselineNodeRatio}`,
            },
          ]
        : [],
    );

  const payload = {
    ok: failures.length === 0,
    type: 'node_critical_path_inventory',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    package_path: args.packagePath,
    baseline_path: args.baselinePath,
    summary: {
      critical_scripts_total: rows.length,
      critical_scripts_missing: missing.length,
      rust_native_count: rustNativeCount,
      node_typescript_count: nodeTypescriptCount,
      npm_wrapper_count: npmWrapperCount,
      unknown_count: unknownCount,
      node_dependency_ratio: nodeDependencyRatio,
      baseline_available: baselineAvailable,
      baseline_node_dependency_ratio: baselineAvailable && Number.isFinite(baselineNodeRatio) ? baselineNodeRatio : null,
      node_dependency_ratio_regression: nodeDependencyRegression,
    },
    rows,
    failures,
    artifact_paths: [args.markdownOutPath],
  };

  writeTextArtifact(args.markdownOutPath, markdown(payload));
  return emitStructuredResult(payload, {
    outPath: args.outPath,
    strict: args.strict,
    ok: payload.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
};
