#!/usr/bin/env tsx

import { createHash } from 'node:crypto';
import fs from 'node:fs';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult, writeJsonArtifact } from '../../lib/result.ts';
import { run as runHarness } from './runtime_proof_harness.ts';
import { run as runReleaseGate } from './runtime_proof_release_gate.ts';
import { run as runAdapterChaosGate } from './adapter_runtime_chaos_gate.ts';
import { run as runBoundednessInspect } from './runtime_boundedness_inspect.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';
type ProfileSelector = ProfileId | 'all';
type ProofTrackId = 'synthetic' | 'empirical' | 'dual';

function parseProfile(raw: string | undefined): ProfileSelector | null {
  const normalized = cleanText(raw || 'all', 32).toLowerCase();
  if (normalized === 'all') return 'all';
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') {
    return 'tiny-max';
  }
  return null;
}

function parseProofTrack(raw: string | undefined): ProofTrackId {
  const normalized = cleanText(raw || 'dual', 24).toLowerCase();
  if (normalized === 'synthetic') return 'synthetic';
  if (normalized === 'empirical') return 'empirical';
  return 'dual';
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
    proofTrack: parseProofTrack(readFlag(argv, 'proof-track')),
  };
}

function readJsonBestEffort(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function sha256File(filePath: string): string {
  try {
    return createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
  } catch {
    return '';
  }
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
      allowed_profiles: ['all', 'rich', 'pure', 'tiny-max'],
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const profiles: ProfileId[] =
    args.profile === 'all' ? (['rich', 'pure', 'tiny-max'] as ProfileId[]) : [args.profile];
  const boundednessInspectOut = 'core/local/artifacts/runtime_boundedness_inspect_current.json';

  const profileRuns = profiles.map((profile) => {
    const harnessOut = `core/local/artifacts/runtime_proof_harness_${profile}_current.json`;
    const harnessMetricsOut = `core/local/artifacts/runtime_proof_metrics_${profile}_current.json`;
    const gateOut = `core/local/artifacts/runtime_proof_release_gate_${profile}_current.json`;
    const gateMetricsOut = `core/local/artifacts/runtime_proof_release_metrics_${profile}_current.json`;
    const gateTableOut = `local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_${profile.toUpperCase()}_CURRENT.md`;
    const adapterChaosOut = `core/local/artifacts/adapter_runtime_chaos_gate_${profile}_current.json`;

    const harnessExit = runHarness([
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--out=${harnessOut}`,
      `--metrics-out=${harnessMetricsOut}`,
    ]);
    const boundednessInspectExit = runBoundednessInspect([
      '--strict=0',
      `--profile=${profile}`,
      `--metrics=${harnessMetricsOut}`,
      `--out=${boundednessInspectOut}`,
    ]);
    const adapterChaosExit = runAdapterChaosGate([
      '--strict=1',
      `--profile=${profile}`,
      `--out=${adapterChaosOut}`,
    ]);
    const gateExit = runReleaseGate([
      '--strict=1',
      `--profile=${profile}`,
      `--proof-track=${args.proofTrack}`,
      `--harness=${harnessOut}`,
      `--adapter-chaos=${adapterChaosOut}`,
      '--policy=tests/tooling/config/release_gates.yaml',
      `--out=${gateOut}`,
      `--metrics-out=${gateMetricsOut}`,
      `--table-out=${gateTableOut}`,
    ]);

    const harnessPayload = readJsonBestEffort(harnessOut);
    const gatePayload = readJsonBestEffort(gateOut);
    const empiricalSamplePoints = Number(harnessPayload?.proof_tracks?.empirical?.sample_points || 0);
    const empiricalRequired = args.proofTrack === 'empirical' || args.proofTrack === 'dual';
    const empiricalGateOk = !empiricalRequired || empiricalSamplePoints > 0;
    const boundednessScenario = Array.isArray(harnessPayload?.scenarios)
      ? harnessPayload.scenarios.find((row: any) => cleanText(row?.id || '', 80) === 'boundedness_72h')
      : null;
    const soakSource = Array.isArray(harnessPayload?.proof_tracks?.empirical?.sources)
      ? harnessPayload.proof_tracks.empirical.sources.find(
          (row: any) => cleanText(row?.id || '', 120) === 'ops_ipc_bridge_stability_soak',
        )
      : null;

    return {
      profile,
      harnessExit,
      boundednessInspectExit,
      gateExit,
      adapterChaosExit,
      empiricalGateOk,
      empiricalSamplePoints,
      harnessOut,
      harnessMetricsOut,
      gateOut,
      gateMetricsOut,
      gateTableOut,
      adapterChaosOut,
      harnessPayload,
      gatePayload,
      boundednessScenario,
      soakSource,
      ok:
        harnessExit === 0 &&
        gateExit === 0 &&
        adapterChaosExit === 0 &&
        empiricalGateOk,
    };
  });

  const boundednessOut = 'core/local/artifacts/runtime_boundedness_72h_evidence_current.json';
  const multiDaySoakOut = 'core/local/artifacts/runtime_multi_day_soak_evidence_current.json';
  const proofChecksumsOut = 'core/local/artifacts/release_proof_checksums_current.json';

  const boundednessEvidence = {
    ok: profileRuns.every((row) => !!row.boundednessScenario && row.boundednessScenario.ok === true),
    type: 'runtime_boundedness_72h_evidence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      scenario_present: !!row.boundednessScenario,
      scenario_ok: row.boundednessScenario?.ok === true,
      metrics: row.boundednessScenario?.metrics || {},
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(boundednessOut, boundednessEvidence);

  const multiDaySoakEvidence = {
    ok: profileRuns.every((row) => row.empiricalSamplePoints > 0 && (row.soakSource?.loaded === true || row.soakSource?.sample_points > 0)),
    type: 'runtime_multi_day_soak_evidence',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    proof_track: args.proofTrack,
    profiles: profileRuns.map((row) => ({
      profile: row.profile,
      empirical_sample_points: row.empiricalSamplePoints,
      soak_source_loaded: row.soakSource?.loaded === true,
      soak_source_sample_points: Number(row.soakSource?.sample_points || 0),
      soak_source_detail: cleanText(row.soakSource?.detail || 'missing', 200),
      source_artifact: row.harnessOut,
    })),
  };
  writeJsonArtifact(multiDaySoakOut, multiDaySoakEvidence);

  const checksumRows = profileRuns
    .flatMap((row) => [
      row.harnessOut,
      row.harnessMetricsOut,
      row.gateOut,
      row.gateMetricsOut,
      row.adapterChaosOut,
      row.gateTableOut,
    ])
    .concat([boundednessOut, multiDaySoakOut])
    .map((artifactPath) => ({
      path: artifactPath,
      exists: fs.existsSync(artifactPath),
      sha256: sha256File(artifactPath),
    }));
  const proofChecksums = {
    ok: checksumRows.every((row) => row.exists && row.sha256.length > 0),
    type: 'release_proof_checksums',
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    checksums: checksumRows,
  };
  writeJsonArtifact(proofChecksumsOut, proofChecksums);

  const ok = profileRuns.every((row) => row.ok) && boundednessEvidence.ok && multiDaySoakEvidence.ok && proofChecksums.ok;
  const artifactPaths = profileRuns
    .flatMap((row) => [
      row.harnessOut,
      row.harnessMetricsOut,
      row.gateOut,
      row.gateMetricsOut,
      row.gateTableOut,
      row.adapterChaosOut,
    ])
    .concat([boundednessInspectOut, boundednessOut, multiDaySoakOut, proofChecksumsOut]);

  const report = {
    ok,
    type: 'runtime_proof_verify',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    summary: {
      pass: ok,
      proof_track: args.proofTrack,
      profile_count: profileRuns.length,
      profiles_passed: profileRuns.filter((row) => row.ok).length,
      boundedness_inspect_exit_max: Math.max(...profileRuns.map((row) => row.boundednessInspectExit)),
    },
    profile_runs: profileRuns.map((row) => ({
      profile: row.profile,
      ok: row.ok,
      harness_exit: row.harnessExit,
      release_gate_exit: row.gateExit,
      adapter_runtime_chaos_exit: row.adapterChaosExit,
      empirical_sample_points: row.empiricalSamplePoints,
      empirical_sample_points_ok: row.empiricalGateOk,
      artifact_paths: [
        row.harnessOut,
        row.harnessMetricsOut,
        row.gateOut,
        row.gateMetricsOut,
        row.gateTableOut,
        row.adapterChaosOut,
      ],
    })),
    evidence: {
      boundedness_72h: boundednessOut,
      multi_day_soak: multiDaySoakOut,
      proof_checksums: proofChecksumsOut,
    },
    artifact_paths: artifactPaths,
    failures: [
      ...profileRuns.flatMap((row) => [
        ...(row.boundednessInspectExit === 0
          ? []
          : [{ id: 'runtime_boundedness_inspect_failed', detail: `profile=${row.profile};exit_code=${row.boundednessInspectExit}` }]),
        ...(row.harnessExit === 0
          ? []
          : [{ id: 'runtime_proof_harness_failed', detail: `profile=${row.profile};exit_code=${row.harnessExit}` }]),
        ...(row.gateExit === 0
          ? []
          : [{ id: 'runtime_proof_release_gate_failed', detail: `profile=${row.profile};exit_code=${row.gateExit}` }]),
        ...(row.adapterChaosExit === 0
          ? []
          : [{ id: 'adapter_runtime_chaos_gate_failed', detail: `profile=${row.profile};exit_code=${row.adapterChaosExit}` }]),
        ...(row.empiricalGateOk
          ? []
          : [{ id: 'runtime_proof_empirical_sample_points_missing', detail: `profile=${row.profile};sample_points=${row.empiricalSamplePoints}` }]),
      ]),
      ...(boundednessEvidence.ok ? [] : [{ id: 'runtime_boundedness_72h_evidence_incomplete', detail: boundednessOut }]),
      ...(multiDaySoakEvidence.ok ? [] : [{ id: 'runtime_multi_day_soak_evidence_incomplete', detail: multiDaySoakOut }]),
      ...(proofChecksums.ok ? [] : [{ id: 'release_proof_checksums_incomplete', detail: proofChecksumsOut }]),
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
