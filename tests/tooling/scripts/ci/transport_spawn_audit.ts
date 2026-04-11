#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { parseBool, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type Row = {
  file: string;
  kind: 'spawn' | 'spawnSync';
  classification:
    | 'dev_only_fallback'
    | 'acceptable_cli_entrypoint'
    | 'external_tool_invocation'
    | 'runtime_process_supervisor'
    | 'test_harness_subprocess'
    | 'wrapper_candidate'
    | 'runtime_hot_path';
  severity: 'info' | 'warn' | 'critical';
  detail: string;
  recommended_action: string;
};

type Args = {
  strict: boolean;
  out: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/transport_spawn_audit_current.json');
const SCAN_ROOTS = ['adapters', 'client', 'tests', 'packages'];

function parseArgs(argv: string[]): Args {
  const rawOut = String(readFlag(argv, 'out') || DEFAULT_OUT).trim();
  return {
    strict: argv.includes('--strict') || parseBool(readFlag(argv, 'strict'), false),
    out: path.resolve(ROOT, rawOut),
  };
}

function rel(filePath: string): string {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function walk(baseRel: string): string[] {
  const base = path.join(ROOT, baseRel);
  if (!fs.existsSync(base)) return [];
  const out: string[] = [];
  const stack = [base];
  while (stack.length > 0) {
    const current = stack.pop() as string;
    for (const entry of fs.readdirSync(current, { withFileTypes: true })) {
      const abs = path.join(current, entry.name);
      const rp = rel(abs);
      if (rp.includes('/node_modules/') || rp.includes('/target/') || rp.includes('/dist/')) continue;
      if (entry.isDirectory()) stack.push(abs);
      else if (entry.isFile() && /\.(ts|tsx|js|mjs|cjs)$/.test(entry.name)) out.push(abs);
    }
  }
  return out;
}

function classify(file: string, source: string): Omit<Row, 'kind'> | null {
  const normalized = file.replace(/\\/g, '/');
  if (!source.includes('spawnSync(') && !source.includes('spawn(')) return null;

  if (normalized.startsWith('adapters/runtime/dev_only/')) {
    return {
      classification: 'dev_only_fallback',
      severity: 'info',
      detail: 'quarantined dev-only fallback',
      recommended_action: 'retain quarantine; do not expose in release paths',
    };
  }

  if (normalized === 'packages/infring-sdk/src/transports.ts') {
    return {
      classification: 'runtime_hot_path',
      severity: 'critical',
      detail: 'sdk transport still uses resident process spawning on fallback path',
      recommended_action: 'continue collapsing toward resident IPC only',
    };
  }

  if (
    normalized === 'adapters/runtime/ops_lane_bridge.ts' ||
    normalized === 'client/runtime/systems/ui/infring_dashboard.ts' ||
    normalized === 'client/runtime/systems/conduit/conduit-client.ts'
  ) {
    return {
      classification: 'runtime_process_supervisor',
      severity: 'info',
      detail: 'runtime/daemon supervision path',
      recommended_action: 'acceptable while process lifecycle remains explicit and receipted',
    };
  }

  if (
    normalized.startsWith('packages/protheus-npm/bin/') ||
    normalized.startsWith('packages/protheus-npm/scripts/')
  ) {
    return {
      classification: 'acceptable_cli_entrypoint',
      severity: 'info',
      detail: 'packaging or CLI entrypoint process handoff',
      recommended_action: 'acceptable CLI boundary; keep outside hot runtime paths',
    };
  }

  if (
    /spawnSync\(\s*['"](cargo|git|bash|which|esbuild|docker)['"]/.test(source) ||
    normalized.includes('benchmark_matrix_refresh.ts') ||
    normalized.includes('reliability_turn_loop_gauntlet.ts')
  ) {
    return {
      classification: 'external_tool_invocation',
      severity: 'info',
      detail: 'invokes external toolchain or benchmark workload intentionally',
      recommended_action: 'acceptable if purpose is real external execution rather than wrapper delegation',
    };
  }

  if (normalized.startsWith('tests/client-memory-tools/') || normalized.startsWith('tests/vitest/')) {
    return {
      classification: 'test_harness_subprocess',
      severity: 'info',
      detail: 'test harness subprocess for runtime or CLI verification',
      recommended_action: 'acceptable if exercising a real external/runtime boundary',
    };
  }

  if (
    normalized.startsWith('tests/tooling/scripts/ci/') ||
    normalized.startsWith('tests/tooling/scripts/ops/') ||
    normalized.startsWith('tests/tooling/scripts/metrics/') ||
    normalized.startsWith('client/runtime/systems/ops/') ||
    normalized.startsWith('client/runtime/systems/autonomy/')
  ) {
    const wrapperLike =
      /spawnSync\(\s*process\.execPath/.test(source) ||
      /spawnSync\(\s*['"]node['"]/.test(source) ||
      /spawn\(\s*process\.execPath/.test(source);
    if (wrapperLike) {
      return {
        classification: 'wrapper_candidate',
        severity: 'warn',
        detail: 'pure wrapper shelling can likely be replaced with in-process delegate or resident bridge',
        recommended_action: 'collapse to invokeTsModuleSync or invokeProtheusOpsViaBridge',
      };
    }
  }

  return {
    classification: 'external_tool_invocation',
    severity: 'info',
    detail: 'subprocess usage present outside release-critical hot paths',
    recommended_action: 'review during future transport cleanup',
  };
}

function buildRows(): Row[] {
  const rows: Row[] = [];
  for (const base of SCAN_ROOTS) {
    for (const filePath of walk(base)) {
      const file = rel(filePath);
      const source = fs.readFileSync(filePath, 'utf8');
      const baseRow = classify(file, source);
      if (!baseRow) continue;
      const kinds: Array<'spawn' | 'spawnSync'> = [];
      if (source.includes('spawnSync(')) kinds.push('spawnSync');
      if (source.includes('spawn(')) kinds.push('spawn');
      for (const kind of kinds) {
        rows.push({
          file,
          kind,
          ...baseRow,
        });
      }
    }
  }
  return rows.sort((a, b) => a.file.localeCompare(b.file) || a.kind.localeCompare(b.kind));
}

function buildReport() {
  const rows = buildRows();
  const summary = {
    total: rows.length,
    wrapper_candidates: rows.filter((row) => row.classification === 'wrapper_candidate').length,
    runtime_hot_path: rows.filter((row) => row.classification === 'runtime_hot_path').length,
    dev_only_fallback: rows.filter((row) => row.classification === 'dev_only_fallback').length,
  };
  const strictFailures = rows.filter(
    (row) =>
      row.classification === 'wrapper_candidate' &&
        (row.file.startsWith('tests/tooling/scripts/ci/') ||
          row.file.startsWith('tests/tooling/scripts/ops/') ||
          row.file.startsWith('client/runtime/systems/ops/') ||
          row.file.startsWith('client/runtime/systems/autonomy/')),
  );
  return {
    ok: strictFailures.length === 0,
    type: 'transport_spawn_audit',
    generated_at: new Date().toISOString(),
    summary,
    strict_failures: strictFailures.map((row) => ({
      file: row.file,
      kind: row.kind,
      classification: row.classification,
      detail: row.detail,
    })),
    rows,
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const report = buildReport();
  return emitStructuredResult(report, {
    outPath: args.out,
    strict: args.strict,
    ok: report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
