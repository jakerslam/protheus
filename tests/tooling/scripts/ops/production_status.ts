#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { parseStrictOutArgs, cleanText } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { invokeTsModuleSync } from '../../../../client/runtime/lib/in_process_ts_delegate.ts';

const ROOT = process.cwd();
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/production_status_current.json');
const PROBE_DIR = path.join(ROOT, 'core/local/artifacts/support_bundle_probes');
const TOPOLOGY_SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ops/production_topology_diagnostic.ts');
const CLOSURE_SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ci/production_readiness_closure_gate.ts');
const VERDICT_SCRIPT = path.join(ROOT, 'tests/tooling/scripts/ci/release_verdict_gate.ts');
const TOPOLOGY_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/production_topology_diagnostic_current.json');
const CLOSURE_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/production_readiness_closure_gate_current.json');
const VERDICT_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/release_verdict_current.json');
const RC_ARTIFACT_PATH = path.join(ROOT, 'core/local/artifacts/release_candidate_dress_rehearsal_current.json');
const RUNTIME_VERSION_PATH = path.join(ROOT, 'client/runtime/config/runtime_version.json');
const RELEASE_CHANNEL_POLICY_PATH = path.join(ROOT, 'client/runtime/config/release_channel_policy.json');
const RELEASE_COMPATIBILITY_POLICY_PATH = path.join(
  ROOT,
  'client/runtime/config/release_compatibility_policy.json',
);
const ASSIMILATION_SUPPORT_PATH = path.join(
  ROOT,
  'client/runtime/config/assimilation_v1_support_contract.json',
);

function parseArgs(argv: string[]) {
  const parsed = parseStrictOutArgs(argv, { out: DEFAULT_OUT, strict: false });
  return {
    strict: parsed.strict,
    out: parsed.out || DEFAULT_OUT,
  };
}

function readJson<T>(filePath: string, fallback: T): T {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8')) as T;
  } catch {
    return fallback;
  }
}

function parseJsonLine(stdout: string): any {
  const lines = String(stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let index = lines.length - 1; index >= 0; index -= 1) {
    try {
      return JSON.parse(lines[index]);
    } catch {}
  }
  return null;
}

function runTs(scriptPath: string, args: string[]) {
  const result = invokeTsModuleSync(scriptPath, {
    argv: args,
    cwd: ROOT,
    exportName: 'run',
    teeStdout: false,
    teeStderr: false,
  });
  return {
    ok: Number(result.status) === 0,
    payload: parseJsonLine(String(result.stdout || '')),
    stderr: cleanText(result.stderr || '', 400),
  };
}

function latestRepoTag(): string {
  const out = spawnSync('git', ['describe', '--tags', '--abbrev=0'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'ignore'],
  });
  return out.status === 0 ? cleanText(out.stdout || '', 120) : '';
}

function hasStructuredPayload(value: unknown): boolean {
  return Boolean(value && typeof value === 'object' && Object.keys(value as Record<string, unknown>).length > 0);
}

export function buildReport(rawArgs = parseArgs(process.argv.slice(2))) {
  const args = typeof rawArgs === 'object' && rawArgs ? rawArgs : parseArgs(process.argv.slice(2));
  const topologyArtifact = readJson<any>(TOPOLOGY_ARTIFACT_PATH, {});
  const closureArtifact = readJson<any>(CLOSURE_ARTIFACT_PATH, {});
  const verdictArtifact = readJson<any>(VERDICT_ARTIFACT_PATH, {});
  const topologyProbe = hasStructuredPayload(topologyArtifact)
    ? { ok: topologyArtifact?.ok === true, payload: topologyArtifact, stderr: '' }
    : runTs(TOPOLOGY_SCRIPT, [`--out=${path.join(PROBE_DIR, 'production_status_topology_probe.json')}`]);
  const closureProbe = hasStructuredPayload(closureArtifact)
    ? { ok: closureArtifact?.summary?.pass === true || closureArtifact?.ok === true, payload: closureArtifact, stderr: '' }
    : runTs(CLOSURE_SCRIPT, [
        '--strict=0',
        '--run-smoke=0',
        '--stage=final',
        `--out=${path.join(PROBE_DIR, 'production_status_closure_probe.json')}`,
      ]);
  const verdictProbe = hasStructuredPayload(verdictArtifact)
    ? { ok: verdictArtifact?.summary?.release_ready === true || verdictArtifact?.ok === true, payload: verdictArtifact, stderr: '' }
    : runTs(VERDICT_SCRIPT, [
        '--strict=0',
        `--out=${path.join(PROBE_DIR, 'production_status_release_verdict_probe.json')}`,
      ]);
  const rc = readJson<any>(RC_ARTIFACT_PATH, {});
  const runtimeVersion = readJson<any>(RUNTIME_VERSION_PATH, {});
  const releaseChannelPolicy = readJson<any>(RELEASE_CHANNEL_POLICY_PATH, {});
  const releaseCompatibilityPolicy = readJson<any>(RELEASE_COMPATIBILITY_POLICY_PATH, {});
  const assimilationSupport = readJson<any>(ASSIMILATION_SUPPORT_PATH, {});
  const repoTag = latestRepoTag();
  const topologySupported = topologyProbe.payload?.supported_production_topology === true;
  const closurePass = closureProbe.payload?.summary?.pass === true || closureProbe.payload?.ok === true;
  const releaseReady = verdictProbe.payload?.summary?.release_ready === true;
  const candidateReady = rc?.summary?.candidate_ready === true;
  const degradedFlags = [
    ...(Array.isArray(topologyProbe.payload?.degraded_flags) ? topologyProbe.payload.degraded_flags : []),
    ...(Array.isArray(closureProbe.payload?.failed_ids) ? closureProbe.payload.failed_ids : []),
    ...(Array.isArray(verdictProbe.payload?.failed_ids) ? verdictProbe.payload.failed_ids : []),
  ].filter(Boolean);
  const report = {
    ok: topologySupported && closurePass && releaseReady,
    type: 'production_status',
    generated_at: new Date().toISOString(),
    summary: {
      topology_supported: topologySupported,
      closure_pass: closurePass,
      release_ready: releaseReady,
      candidate_ready: candidateReady,
      degraded_flag_count: degradedFlags.length,
    },
    topology: {
      support_level: cleanText(topologyProbe.payload?.support_level || '', 80),
      topology_mode: cleanText(topologyProbe.payload?.topology_mode || '', 120),
      degraded_flags: Array.isArray(topologyProbe.payload?.degraded_flags) ? topologyProbe.payload.degraded_flags : [],
    },
    closure: {
      pass: closurePass,
      failed_ids: Array.isArray(closureProbe.payload?.failed_ids) ? closureProbe.payload.failed_ids : [],
    },
    release_verdict: {
      release_ready: releaseReady,
      failed_ids: Array.isArray(verdictProbe.payload?.failed_ids) ? verdictProbe.payload.failed_ids : [],
      verdict_checksum: cleanText(verdictProbe.payload?.verdict_checksum || '', 120),
    },
    last_rehearsal: {
      present: fs.existsSync(RC_ARTIFACT_PATH),
      candidate_ready: candidateReady,
      required_steps_satisfied: rc?.summary?.required_steps_satisfied === true,
      generated_at: cleanText(rc?.generated_at || '', 80),
      failed_count: Number(rc?.summary?.failed_count || 0),
    },
    version: {
      declared_version: cleanText(runtimeVersion?.version || '', 80),
      declared_tag: cleanText(runtimeVersion?.tag || '', 120),
      declared_release_channel: cleanText(runtimeVersion?.release_channel || '', 40),
      default_release_channel: cleanText(releaseChannelPolicy?.default_channel || '', 40),
      latest_repo_tag: repoTag,
      runtime_version_matches_repo_tag:
        Boolean(repoTag) && cleanText(runtimeVersion?.tag || '', 120) === repoTag,
      compatibility_policy_present: fs.existsSync(RELEASE_COMPATIBILITY_POLICY_PATH),
      compatibility_registry_path: cleanText(releaseCompatibilityPolicy?.registry_path || '', 200),
    },
    support_contract: {
      canonical_surface: cleanText(topologyProbe.payload?.surface_contract?.canonical_surface || '', 80),
      production_supported_commands: Array.isArray(topologyProbe.payload?.surface_contract?.production_supported)
        ? topologyProbe.payload.surface_contract.production_supported
        : [],
      assimilation_support_level: cleanText(
        assimilationSupport?.production_contract?.support_level || '',
        80,
      ),
      assimilation_release_supported: assimilationSupport?.production_contract?.release_supported === true,
    },
    degraded_flags: degradedFlags,
  };
  return {
    outPath: path.resolve(args.out || DEFAULT_OUT),
    strict: Boolean(args.strict),
    report,
  };
}

export function run(argv = process.argv.slice(2)) {
  const result = buildReport(parseArgs(argv));
  return emitStructuredResult(result.report, {
    outPath: result.outPath,
    strict: result.strict,
    ok: result.report.ok,
  });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  buildReport,
  run,
};
