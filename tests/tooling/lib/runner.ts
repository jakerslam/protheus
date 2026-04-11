#!/usr/bin/env node
'use strict';

import { toolingLatestPath } from './artifacts.ts';
import { loadGateRegistry, loadVerifyProfiles, ToolingGate, ToolingProfileManifest } from './manifest.ts';
import { expandEnvValue, runCommand } from './process.ts';
import { writeJsonArtifact } from './result.ts';

export const DEFAULT_GATE_REGISTRY_PATH = 'tests/tooling/config/tooling_gate_registry.json';
export const DEFAULT_VERIFY_PROFILES_PATH = 'tests/tooling/config/verify_profiles.json';

export type GateExecutionReport = {
  ok: boolean;
  type: 'tooling_gate_run';
  generated_at: string;
  duration_ms: number;
  owner: string;
  gate_id: string;
  description: string;
  inputs: {
    registry_path: string;
    strict: boolean;
    script: string | null;
    command: string[];
    timeout_sec: number;
    defer_host_stall: boolean;
  };
  summary: {
    pass: boolean;
    exit_code: number;
    signal: string | null;
    timed_out: boolean;
    deferred_host_stall: boolean;
  };
  failures: Array<{ id: string; detail: string }>;
  artifact_paths: string[];
  stdout: string;
  stderr: string;
};

function effectiveTimeout(gate: ToolingGate): number {
  const envKey = String(gate.timeout_env || '').trim();
  const envValue = envKey ? Number(process.env[envKey] || '') : Number.NaN;
  if (Number.isFinite(envValue) && envValue > 0) return Math.floor(envValue);
  return Math.max(1, Math.floor(Number(gate.timeout_sec || 60)));
}

function expandedCommand(gate: ToolingGate): string[] {
  if (gate.script) return ['npm', 'run', '-s', gate.script];
  return (gate.command || []).map((part) => expandEnvValue(part, process.env));
}

export function resolveGate(
  gateId: string,
  registryPath = DEFAULT_GATE_REGISTRY_PATH,
): ToolingGate {
  const registry = loadGateRegistry(registryPath);
  const gate = registry.gates?.[gateId];
  if (!gate) throw new Error(`tooling_gate_missing:${gateId}`);
  return {
    id: gateId,
    ...gate,
  };
}

export function executeGate(
  gateId: string,
  options: {
    registryPath?: string;
    strict?: boolean;
    outPath?: string;
  } = {},
): GateExecutionReport {
  const registryPath = options.registryPath || DEFAULT_GATE_REGISTRY_PATH;
  const gate = resolveGate(gateId, registryPath);
  const timeoutSec = effectiveTimeout(gate);
  const command = expandedCommand(gate);
  const started = Date.now();
  const processResult = runCommand(command, {
    cwd: process.cwd(),
    env: process.env,
    timeoutSec,
    deferHostStall: Boolean(gate.defer_host_stall),
  });
  const failures =
    processResult.ok
      ? []
      : [
          {
            id: gateId,
            detail:
              `status=${processResult.status}; signal=${String(processResult.signal || '')}; ` +
              `${String(processResult.stderr || processResult.stdout).trim().slice(0, 500)}`,
          },
        ];
  const artifactPaths = [
    ...(gate.artifact_paths || []).map((value) => expandEnvValue(value, process.env)).filter(Boolean),
  ];
  const report: GateExecutionReport = {
    ok: failures.length === 0,
    type: 'tooling_gate_run',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: String(gate.owner || 'tooling'),
    gate_id: gateId,
    description: String(gate.description || ''),
    inputs: {
      registry_path: registryPath,
      strict: Boolean(options.strict),
      script: gate.script || null,
      command,
      timeout_sec: timeoutSec,
      defer_host_stall: Boolean(gate.defer_host_stall),
    },
    summary: {
      pass: failures.length === 0,
      exit_code: processResult.status,
      signal: processResult.signal,
      timed_out: processResult.timed_out,
      deferred_host_stall: processResult.deferred_host_stall,
    },
    failures,
    artifact_paths: artifactPaths,
    stdout: processResult.stdout,
    stderr: processResult.stderr,
  };
  writeJsonArtifact(options.outPath || toolingLatestPath('gate', gateId), report);
  return report;
}

export function collectRegistrySummary(
  registryPath = DEFAULT_GATE_REGISTRY_PATH,
  profilesPath = DEFAULT_VERIFY_PROFILES_PATH,
) {
  const registry = loadGateRegistry(registryPath);
  const profiles = loadVerifyProfiles(profilesPath);
  return {
    ok: true,
    type: 'tooling_registry_list',
    generated_at: new Date().toISOString(),
    inputs: {
      registry_path: registryPath,
      profiles_path: profilesPath,
    },
    summary: {
      gate_count: Object.keys(registry.gates || {}).length,
      profile_count: Object.keys(profiles.profiles || {}).length,
    },
    gates: Object.entries(registry.gates || {})
      .map(([id, gate]) => ({
        id,
        owner: gate.owner,
        description: gate.description,
        script: gate.script || null,
        timeout_sec: effectiveTimeout({ id, ...gate }),
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
    profiles: Object.entries(profiles.profiles || {})
      .map(([id, profile]) => ({
        id,
        description: profile.description,
        gate_ids: profile.gate_ids,
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
  };
}

export function executeProfile(
  profileId: string,
  options: {
    registryPath?: string;
    profilesPath?: string;
    strict?: boolean;
    outPath?: string;
  } = {},
) {
  const registryPath = options.registryPath || DEFAULT_GATE_REGISTRY_PATH;
  const profilesPath = options.profilesPath || DEFAULT_VERIFY_PROFILES_PATH;
  const profiles = loadVerifyProfiles(profilesPath);
  const profile = profiles.profiles?.[profileId];
  if (!profile) throw new Error(`tooling_profile_missing:${profileId}`);
  const started = Date.now();
  const gateReports = profile.gate_ids.map((gateId) =>
    executeGate(gateId, {
      registryPath,
      strict: options.strict,
    }),
  );
  const failed = gateReports.filter((row) => !row.ok);
  const report = {
    ok: failed.length === 0,
    type: 'tooling_profile_run',
    generated_at: new Date().toISOString(),
    duration_ms: Date.now() - started,
    owner: 'tooling',
    profile_id: profileId,
    description: profile.description,
    inputs: {
      registry_path: registryPath,
      profiles_path: profilesPath,
      strict: Boolean(options.strict),
    },
    summary: {
      gate_count: gateReports.length,
      failed_count: failed.length,
      pass: failed.length === 0,
    },
    failures: failed.map((row) => ({
      id: row.gate_id,
      detail: row.failures[0]?.detail || 'gate_failed',
    })),
    artifact_paths: gateReports.flatMap((row) => row.artifact_paths),
    gates: gateReports.map((row) => ({
      gate_id: row.gate_id,
      ok: row.ok,
      owner: row.owner,
      duration_ms: row.duration_ms,
      exit_code: row.summary.exit_code,
      artifact_paths: row.artifact_paths,
    })),
  };
  writeJsonArtifact(options.outPath || toolingLatestPath('profile', profileId), report);
  return report;
}

