#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';

type Args = {
  strict: boolean;
  out: string;
};

type Check = {
  id: string;
  ok: boolean;
  detail: string;
};

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/assimilation_v1_support_guard_current.json');
const CONTRACT_PATH = path.join(ROOT, 'client/runtime/config/assimilation_v1_support_contract.json');
const README_PATH = path.join(ROOT, 'README.md');
const TEST_SOURCE_PATH = path.join(
  ROOT,
  'core/layer0/ops/src/runtime_systems_parts/120-run-writes-latest-and-status-reads-it.rs',
);

function parseArgs(argv: string[]): Args {
  const args: Args = { strict: false, out: DEFAULT_OUT };
  for (const token of argv) {
    if (token === '--strict' || token === '--strict=1') args.strict = true;
    else if (token.startsWith('--strict=')) {
      const value = token.slice('--strict='.length).trim().toLowerCase();
      args.strict = ['1', 'true', 'yes', 'on'].includes(value);
    } else if (token.startsWith('--out=')) {
      const raw = token.slice('--out='.length).trim();
      if (raw) args.out = path.resolve(ROOT, raw);
    }
  }
  return args;
}

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function readJson(filePath: string, fallback: any = null): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function readText(filePath: string): string {
  try {
    return fs.readFileSync(filePath, 'utf8');
  } catch {
    return '';
  }
}

function buildReport() {
  const contract = readJson(CONTRACT_PATH, {});
  const readme = readText(README_PATH);
  const tests = readText(TEST_SOURCE_PATH);
  const supportedComponents = Array.isArray(contract?.canonical_slice?.supported_components)
    ? contract.canonical_slice.supported_components.map((row: unknown) => clean(row, 120)).filter(Boolean)
    : [];
  const evidencePaths = Array.isArray(contract?.required_evidence) ? contract.required_evidence : [];
  const componentFiles = [
    'client/runtime/systems/assimilation/source_attestation_extension.ts',
    'client/runtime/systems/assimilation/trajectory_skill_distiller.ts',
    'client/runtime/systems/assimilation/world_model_freshness.ts',
  ];
  const checks: Check[] = [
    {
      id: 'contract_status',
      ok: clean(contract?.status, 80) === 'frozen_v1_vertical_slice',
      detail: `status=${clean(contract?.status, 80) || 'missing'}`,
    },
    {
      id: 'support_level',
      ok:
        clean(contract?.production_contract?.support_level, 80) === 'experimental_opt_in' &&
        contract?.production_contract?.release_supported === false,
      detail: `support_level=${clean(contract?.production_contract?.support_level, 80) || 'missing'};release_supported=${String(contract?.production_contract?.release_supported)}`,
    },
    {
      id: 'canonical_slice_name',
      ok: clean(contract?.canonical_slice?.name, 120) === 'runtime_ingress_to_assimilation_kernel',
      detail: `name=${clean(contract?.canonical_slice?.name, 120) || 'missing'}`,
    },
    {
      id: 'supported_modes',
      ok:
        Array.isArray(contract?.canonical_slice?.supported_modes) &&
        contract.canonical_slice.supported_modes.includes('plan_only') &&
        contract.canonical_slice.supported_modes.includes('admitted_execution'),
      detail: `modes=${JSON.stringify(contract?.canonical_slice?.supported_modes || [])}`,
    },
    {
      id: 'supported_components',
      ok:
        supportedComponents.length === 3 &&
        ['source_attestation_extension', 'trajectory_skill_distiller', 'world_model_freshness'].every((row) =>
          supportedComponents.includes(row),
        ),
      detail: `components=${supportedComponents.join(',') || 'missing'}`,
    },
    {
      id: 'evidence_paths',
      ok: evidencePaths.length >= 3 && evidencePaths.every((row: string) => fs.existsSync(path.join(ROOT, row))),
      detail: `evidence=${evidencePaths.length}`,
    },
    {
      id: 'component_files',
      ok: componentFiles.every((row) => fs.existsSync(path.join(ROOT, row))),
      detail: componentFiles.map((row) => `${row}:${fs.existsSync(path.join(ROOT, row))}`).join('; '),
    },
    {
      id: 'readme_markers',
      ok:
        readme.includes('Experimental lanes (explicit opt-in): `assimilate`') &&
        readme.includes('Frozen assimilation v1 slice: one ingress -> orchestration -> assimilation-kernel -> receipt-output path is hardened; broader assimilation surfaces remain experimental.') &&
        readme.includes('Use `--plan-only=1` to emit the canonical assimilation planning chain without executing bridge mutations.'),
      detail: 'README support matrix + plan_only marker',
    },
    {
      id: 'runtime_tests_present',
      ok:
        tests.includes('assimilation_lane_emits_protocol_summary_and_artifacts') &&
        tests.includes('assimilation_lane_hard_selector_cannot_bypass_closure') &&
        tests.includes('assimilation_lane_selector_bypass_rejected_under_strict_mode') &&
        tests.includes('assimilation_lane_strict_rejects_unknown_operation'),
      detail: 'runtime_systems assimilation test markers present',
    },
  ];
  const failed = checks.filter((row) => !row.ok);
  return {
    ok: failed.length === 0,
    type: 'assimilation_v1_support_guard',
    generated_at: new Date().toISOString(),
    failed_ids: failed.map((row) => row.id),
    checks,
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = parseArgs(argv);
  const report = buildReport();
  fs.mkdirSync(path.dirname(args.out), { recursive: true });
  fs.writeFileSync(args.out, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  console.log(JSON.stringify(report, null, 2));
  if (args.strict && report.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
