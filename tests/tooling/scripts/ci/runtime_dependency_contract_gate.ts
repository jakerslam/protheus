import fs from 'node:fs';
import { createRequire } from 'node:module';
import { cleanText, hasFlag, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type GatePayload = {
  ok: boolean;
  type: 'runtime_dependency_contract_gate';
  generated_at: string;
  revision: string;
  strict: boolean;
  inputs: {
    out: string;
  };
  summary: {
    required_modules: number;
    manifest_entries: number;
    tier1_runtime_entries: number;
  };
  failures: string[];
  warnings: string[];
};

const ROOT = process.cwd();
const nodeRequire = createRequire(`${ROOT}/package.json`);
const DEFAULT_OUT = 'core/local/artifacts/runtime_dependency_contract_gate_current.json';

function parseArgs(argv: string[]) {
  return {
    strict:
      hasFlag(argv, 'strict') ||
      ['1', 'true', 'yes', 'on'].includes(cleanText(readFlag(argv, 'strict') || '', 16).toLowerCase()),
    out: cleanText(readFlag(argv, 'out') || DEFAULT_OUT, 400),
  };
}

function readJson(abs: string): any {
  return JSON.parse(fs.readFileSync(abs, 'utf8'));
}

function uniqueSorted(rows: string[]): string[] {
  return [...new Set(rows)].sort((a, b) => a.localeCompare(b));
}

function parseManifestRows(abs: string): string[] {
  return fs
    .readFileSync(abs, 'utf8')
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0 && !line.startsWith('#'));
}

function parseTier1RuntimeEntriesFromKernel(abs: string): string[] {
  const source = fs.readFileSync(abs, 'utf8');
  const blockMatch = source.match(
    /const\s+TIER1_RUNTIME_ENTRYPOINTS:\s*&\[\s*&str\s*\]\s*=\s*&\[(.*?)\];/s,
  );
  if (!blockMatch) return [];
  const rows: string[] = [];
  const literal = /"([^"]+)"/g;
  let match: RegExpExecArray | null = null;
  while ((match = literal.exec(blockMatch[1]))) rows.push(match[1]);
  return rows;
}

function buildReport(strict = false): GatePayload {
  const failures: string[] = [];
  const warnings: string[] = [];
  const pkgPath = `${ROOT}/package.json`;
  let requiredModules: string[] = [];
  let manifestRel = 'client/runtime/config/install_runtime_manifest_v1.txt';

  if (!fs.existsSync(pkgPath)) {
    failures.push('package_json_missing');
  }

  if (failures.length === 0) {
    const pkg = readJson(pkgPath);
    const runtimeContract = pkg?.runtimeDependencyContract ?? {};
    manifestRel = String(runtimeContract?.tier1RuntimeManifest || manifestRel);
    requiredModules = Array.isArray(runtimeContract?.requiredNodeModules)
      ? runtimeContract.requiredNodeModules.map((row: unknown) => String(row))
      : [];
    if (requiredModules.length === 0) failures.push('runtime_dependency_contract_missing_required_modules');

    const deps = pkg?.dependencies ?? {};
    for (const moduleName of requiredModules) {
      if (typeof deps[moduleName] !== 'string' || deps[moduleName].trim().length === 0) {
        failures.push(`runtime_dependency_not_declared:${moduleName}`);
      }
      try {
        nodeRequire.resolve(moduleName, { paths: [ROOT] });
      } catch {
        failures.push(`runtime_dependency_not_resolvable:${moduleName}`);
      }
    }
  }

  const manifestPath = `${ROOT}/${manifestRel}`;
  let manifestRows: string[] = [];
  if (!fs.existsSync(manifestPath)) {
    failures.push(`tier1_manifest_missing:${manifestRel}`);
  } else {
    manifestRows = parseManifestRows(manifestPath);
    if (manifestRows.length === 0) failures.push('tier1_manifest_empty');
    for (const rel of manifestRows) {
      const abs = `${ROOT}/${rel}`;
      if (!fs.existsSync(abs)) failures.push(`tier1_manifest_entry_missing:${rel}`);
    }
  }

  const kernelPath = `${ROOT}/core/layer0/ops/src/command_list_kernel.rs`;
  let tier1KernelEntries: string[] = [];
  if (!fs.existsSync(kernelPath)) {
    failures.push('command_list_kernel_missing');
  } else {
    tier1KernelEntries = parseTier1RuntimeEntriesFromKernel(kernelPath);
    if (tier1KernelEntries.length === 0) failures.push('tier1_runtime_entries_missing_in_kernel');
  }

  if (manifestRows.length > 0 && tier1KernelEntries.length > 0) {
    const left = uniqueSorted(manifestRows);
    const right = uniqueSorted(tier1KernelEntries);
    if (JSON.stringify(left) !== JSON.stringify(right)) {
      failures.push('tier1_runtime_manifest_kernel_mismatch');
      warnings.push(`manifest_only:${left.filter((entry) => !right.includes(entry)).join(',')}`);
      warnings.push(`kernel_only:${right.filter((entry) => !left.includes(entry)).join(',')}`);
    }
  }

  const routesPath = `${ROOT}/core/layer0/ops/src/protheusctl_routes_parts/010-command-routing.rs`;
  if (!fs.existsSync(routesPath)) {
    failures.push('command_routing_source_missing');
  } else {
    const daemonRoutesPath =
      `${ROOT}/core/layer0/ops/src/protheusctl_routes_parts/010-command-routing_parts/001-resolve_core_shortcuts_family_daemon.rs`;
    const source = [
      fs.readFileSync(routesPath, 'utf8'),
      fs.existsSync(daemonRoutesPath) ? fs.readFileSync(daemonRoutesPath, 'utf8') : '',
    ].join('\n');
    if (!source.includes('"dashboard" => Some(route_dashboard_compat(rest, false))')) {
      failures.push('dashboard_not_canonical_core_route');
    }
    if (!source.includes('"dashboard-ui" => Some(route_dashboard_compat(rest, true))')) {
      failures.push('dashboard_ui_legacy_alias_missing');
    }
    if (!source.includes('"gateway" =>')) {
      failures.push('gateway_core_route_missing');
    }
  }

  return {
    ok: failures.length === 0,
    type: 'runtime_dependency_contract_gate',
    generated_at: new Date().toISOString(),
    revision: currentRevision(ROOT),
    strict,
    inputs: {
      out: DEFAULT_OUT,
    },
    summary: {
      required_modules: requiredModules.length,
      manifest_entries: manifestRows.length,
      tier1_runtime_entries: tier1KernelEntries.length,
    },
    failures,
    warnings,
  };
}

function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const payload = {
    ...buildReport(args.strict),
    inputs: {
      out: args.out,
    },
  };
  return emitStructuredResult(payload, {
    outPath: args.out,
    strict: args.strict,
    ok: payload.failures.length === 0,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
