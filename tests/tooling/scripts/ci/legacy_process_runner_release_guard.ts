#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/legacy_process_runner_release_guard_current.json');

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(raw: string | undefined, fallback = false): boolean {
  const value = clean(raw, 24).toLowerCase();
  if (!value) return fallback;
  return value === '1' || value === 'true' || value === 'yes' || value === 'on';
}

function parseArgs(argv: string[]) {
  const parsed = {
    strict: false,
    out: DEFAULT_OUT,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 400);
    if (!token) continue;
    if (token.startsWith('--strict=')) parsed.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--out=')) parsed.out = path.resolve(ROOT, clean(token.slice(6), 400));
  }
  return parsed;
}

function readSource(relPath: string): string {
  const abs = path.join(ROOT, relPath);
  return fs.existsSync(abs) ? fs.readFileSync(abs, 'utf8') : '';
}

function buildReport() {
  const runnerPath = 'adapters/runtime/run_protheus_ops.ts';
  const bridgePath = 'adapters/runtime/ops_lane_bridge.ts';
  const legacyHelperPath = 'adapters/runtime/dev_only/legacy_process_runner.ts';
  const processFallbackHelperPath = 'adapters/runtime/dev_only/ops_lane_process_fallback.ts';
  const runtimeManifestPath = 'client/runtime/config/install_runtime_manifest_v1.txt';

  const runnerSource = readSource(runnerPath);
  const bridgeSource = readSource(bridgePath);
  const legacyHelperSource = readSource(legacyHelperPath);
  const processFallbackHelperSource = readSource(processFallbackHelperPath);
  const runtimeManifest = readSource(runtimeManifestPath);

  const checks = [
    {
      id: 'runner_entrypoint_has_no_spawn_sync',
      ok: !runnerSource.includes('spawnSync('),
      detail: 'run_protheus_ops.ts must stay resident-first',
    },
    {
      id: 'bridge_entrypoint_has_no_spawn_sync',
      ok: !bridgeSource.includes('spawnSync('),
      detail: 'ops_lane_bridge.ts must not embed process fallback execution',
    },
    {
      id: 'runner_entrypoint_uses_dev_only_helper',
      ok: runnerSource.includes("./dev_only/legacy_process_runner.ts"),
      detail: 'legacy runner must be loaded from adapters/runtime/dev_only',
    },
    {
      id: 'bridge_entrypoint_uses_dev_only_helper',
      ok: bridgeSource.includes("./dev_only/ops_lane_process_fallback.ts"),
      detail: 'process fallback helper must be loaded from adapters/runtime/dev_only',
    },
    {
      id: 'legacy_helper_marked_dev_only',
      ok:
        legacyHelperSource.includes('legacy_process_runner_dev_only') &&
        legacyHelperSource.includes('spawnSync('),
      detail: 'legacy helper must be explicitly marked dev-only',
    },
    {
      id: 'process_fallback_helper_marked_dev_only',
      ok:
        processFallbackHelperSource.includes('process_fallback_dev_only') &&
        processFallbackHelperSource.includes('spawnSync('),
      detail: 'process fallback helper must be explicitly marked dev-only',
    },
    {
      id: 'runtime_manifest_excludes_dev_only_helpers',
      ok:
        !runtimeManifest.includes('adapters/runtime/dev_only/') &&
        !runtimeManifest.includes('legacy_process_runner.ts') &&
        !runtimeManifest.includes('ops_lane_process_fallback.ts'),
      detail: 'install runtime manifest must not ship dev-only legacy helpers',
    },
  ];

  return {
    ok: checks.every((row) => row.ok),
    type: 'legacy_process_runner_release_guard',
    generated_at: new Date().toISOString(),
    checks,
    failed: checks.filter((row) => !row.ok).map((row) => row.id),
  };
}

function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const report = buildReport();
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  process.stdout.write(`${JSON.stringify(report)}\n`);
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { buildReport, run };
