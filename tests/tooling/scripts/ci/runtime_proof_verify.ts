#!/usr/bin/env tsx

import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { run as runHarness } from './runtime_proof_harness.ts';
import { run as runReleaseGate } from './runtime_proof_release_gate.ts';
import { run as runAdapterChaosGate } from './adapter_runtime_chaos_gate.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

function parseProfile(raw: string | undefined): ProfileId | null {
  const normalized = cleanText(raw || 'rich', 32).toLowerCase();
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') {
    return 'tiny-max';
  }
  return null;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/runtime_proof_verify_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    profile,
  };
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'runtime_proof_verify',
      error: 'runtime_proof_profile_invalid',
      profile: cleanText(readFlag(argv, 'profile') || '', 40),
      allowed_profiles: ['rich', 'pure', 'tiny-max'],
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const harnessOut = `core/local/artifacts/runtime_proof_harness_${args.profile}_current.json`;
  const harnessMetricsOut = `core/local/artifacts/runtime_proof_metrics_${args.profile}_current.json`;
  const gateOut = `core/local/artifacts/runtime_proof_release_gate_${args.profile}_current.json`;
  const gateMetricsOut = `core/local/artifacts/runtime_proof_release_metrics_${args.profile}_current.json`;
  const gateTableOut = `local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_${args.profile.toUpperCase()}_CURRENT.md`;
  const adapterChaosOut = `core/local/artifacts/adapter_runtime_chaos_gate_${args.profile}_current.json`;

  const harnessExit = runHarness([
    '--strict=1',
    `--profile=${args.profile}`,
    `--out=${harnessOut}`,
    `--metrics-out=${harnessMetricsOut}`,
  ]);
  const gateExit = runReleaseGate([
    '--strict=1',
    `--profile=${args.profile}`,
    `--harness=${harnessOut}`,
    '--policy=tests/tooling/config/release_gates.yaml',
    `--out=${gateOut}`,
    `--metrics-out=${gateMetricsOut}`,
    `--table-out=${gateTableOut}`,
  ]);
  const adapterChaosExit = runAdapterChaosGate([
    '--strict=1',
    `--profile=${args.profile}`,
    `--out=${adapterChaosOut}`,
  ]);

  const ok = harnessExit === 0 && gateExit === 0 && adapterChaosExit === 0;
  const report = {
    ok,
    type: 'runtime_proof_verify',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    summary: {
      pass: ok,
      harness_exit: harnessExit,
      release_gate_exit: gateExit,
      adapter_runtime_chaos_exit: adapterChaosExit,
    },
    artifact_paths: [harnessOut, harnessMetricsOut, gateOut, gateMetricsOut, gateTableOut, adapterChaosOut],
    failures: [
      ...(harnessExit === 0
        ? []
        : [{ id: 'runtime_proof_harness_failed', detail: `exit_code=${harnessExit}` }]),
      ...(gateExit === 0
        ? []
        : [{ id: 'runtime_proof_release_gate_failed', detail: `exit_code=${gateExit}` }]),
      ...(adapterChaosExit === 0
        ? []
        : [{ id: 'adapter_runtime_chaos_gate_failed', detail: `exit_code=${adapterChaosExit}` }]),
    ],
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
