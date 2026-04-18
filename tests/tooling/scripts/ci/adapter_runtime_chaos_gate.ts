#!/usr/bin/env tsx

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

type ProfileId = 'rich' | 'pure' | 'tiny-max';

type AdapterLane = {
  id: string;
  bridgeCommand: string;
  framework: string;
};

type ScenarioRow = {
  adapter: string;
  scenario: string;
  expected_error: string;
  ok: boolean;
  detail: string;
};

type GraduationManifest = {
  version: number;
  required_hooks: string[];
  required_scenarios: string[];
  adapters: Array<{
    id: string;
    framework: string;
    bridge_command: string;
    tier?: string;
  }>;
};

type AdapterGraduationResult = {
  adapter: string;
  graduated: boolean;
  manifest_declared: boolean;
  missing_scenarios: string[];
  hook_pass_count: number;
  hook_total: number;
  hook_pass_ratio: number;
  scenario_pass_count: number;
  scenario_total: number;
  scenario_pass_ratio: number;
  hooks: Array<{ id: string; ok: boolean; detail: string }>;
};

const OPS_CARGO_BUILD_ARGS = ['build', '-q', '-p', 'protheus-ops-core', '--bin', 'protheus-ops'];
const BRIDGE_MAX_BUFFER_BYTES = 64 * 1024 * 1024;

const ADAPTERS: AdapterLane[] = [
  { id: 'langgraph', bridgeCommand: 'workflow_graph-bridge', framework: 'langgraph' },
  { id: 'crewai', bridgeCommand: 'crewai-bridge', framework: 'crewai' },
  { id: 'openai_agents', bridgeCommand: 'pydantic-ai-bridge', framework: 'openai_agents' },
  { id: 'mastra', bridgeCommand: 'mastra-bridge', framework: 'mastra' },
  { id: 'semantic_kernel', bridgeCommand: 'semantic-kernel-bridge', framework: 'semantic_kernel' },
];

const CHAOS_CASES = [
  { id: 'process_never_starts', expected_error: 'adapter_startup_timeout' },
  { id: 'starts_then_hangs', expected_error: 'adapter_request_timeout' },
  { id: 'invalid_schema_response', expected_error: 'adapter_invalid_schema' },
  { id: 'response_too_large', expected_error: 'adapter_response_too_large' },
  { id: 'repeated_flapping', expected_error: 'adapter_circuit_open' },
];

function parseProfile(raw: string | undefined): ProfileId | null {
  const normalized = cleanText(raw || 'rich', 32).toLowerCase();
  if (normalized === 'rich') return 'rich';
  if (normalized === 'pure') return 'pure';
  if (normalized === 'tiny-max' || normalized === 'tiny' || normalized === 'tiny_max') return 'tiny-max';
  return null;
}

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, {
    out: 'core/local/artifacts/adapter_runtime_chaos_gate_current.json',
  });
  const profile = parseProfile(readFlag(argv, 'profile'));
  return {
    strict: common.strict,
    outPath: cleanText(readFlag(argv, 'out') || common.out || '', 400),
    profile,
    graduationManifestPath: cleanText(
      readFlag(argv, 'graduation-manifest') || 'tests/tooling/config/adapter_graduation_manifest.json',
      400,
    ),
  };
}

function parseLastJson(raw: string): any {
  const lines = String(raw || '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  for (let idx = lines.length - 1; idx >= 0; idx -= 1) {
    try {
      return JSON.parse(lines[idx]);
    } catch {
      // keep scanning
    }
  }
  return null;
}

type OpsBinary = {
  command: string;
  argsPrefix: string[];
};

function resolveOpsBinary(root: string): OpsBinary {
  const explicit = cleanText(process.env.INFRING_OPS_BIN || '', 400);
  if (explicit && fs.existsSync(path.resolve(root, explicit))) {
    return { command: path.resolve(root, explicit), argsPrefix: [] };
  }
  const defaultBin = path.resolve(root, 'target/debug/protheus-ops');
  const skipBuild = cleanText(process.env.INFRING_ADAPTER_CHAOS_SKIP_BUILD || '', 8) === '1';
  if (!skipBuild) {
    const build = spawnSync('cargo', OPS_CARGO_BUILD_ARGS, {
      cwd: root,
      encoding: 'utf8',
    });
    if ((build.status ?? 1) !== 0) {
      throw new Error(
        cleanText(
          `adapter_runtime_chaos_build_failed:status=${build.status};stderr=${String(build.stderr || '')}`,
          600,
        ),
      );
    }
  } else if (!fs.existsSync(defaultBin)) {
    throw new Error('adapter_runtime_chaos_binary_missing_when_build_skipped');
  }
  return { command: defaultBin, argsPrefix: [] };
}

function encodePayload(payload: unknown): string {
  return Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
}

function runBridgeCommand(
  root: string,
  opsBinary: OpsBinary,
  adapter: AdapterLane,
  payload: Record<string, unknown>,
  statePath: string,
) {
  const encodedPayload = encodePayload(payload);
  const proc = spawnSync(
    opsBinary.command,
    opsBinary.argsPrefix.concat([
      adapter.bridgeCommand,
      'run-governed-workflow',
      `--payload-base64=${encodedPayload}`,
      `--state-path=${statePath}`,
    ]),
    {
      cwd: root,
      encoding: 'utf8',
      maxBuffer: BRIDGE_MAX_BUFFER_BYTES,
    },
  );
  const spawnError = cleanText(proc.error?.message || '', 280);
  return {
    exitCode: proc.status ?? 1,
    stdout: String(proc.stdout || ''),
    stderr: String(proc.stderr || ''),
    spawnError,
    payload: parseLastJson(String(proc.stdout || '')),
  };
}

function scenarioStatePath(tempRoot: string, adapterId: string, scenarioId: string): string {
  const safeAdapter = adapterId.replace(/[^a-zA-Z0-9_-]/g, '_');
  const safeScenario = scenarioId.replace(/[^a-zA-Z0-9_-]/g, '_');
  return path.join(tempRoot, `${safeAdapter}_${safeScenario}.json`);
}

function baselinePayload(adapter: AdapterLane) {
  return {
    task_id: `adapter_chaos_baseline_${adapter.id}`,
    trace_id: `adapter_chaos_trace_${adapter.id}`,
    tool_name: 'web_search',
    tool_args: {
      query: `adapter chaos baseline ${adapter.id}`,
    },
    raw_result: {
      results: [
        {
          source: `${adapter.id}_runtime`,
          title: `baseline-${adapter.id}`,
          summary: 'baseline governed workflow path',
        },
      ],
    },
  };
}

function chaosPayload(adapter: AdapterLane, scenarioId: string) {
  return {
    task_id: `adapter_chaos_${adapter.id}_${scenarioId}`,
    trace_id: `adapter_chaos_trace_${adapter.id}_${scenarioId}`,
    tool_name: 'web_search',
    chaos_scenario: scenarioId,
    tool_args: {
      query: `adapter chaos ${scenarioId}`,
    },
  };
}

function loadGraduationManifest(root: string, relPath: string): GraduationManifest {
  const abs = path.resolve(root, relPath);
  const parsed = JSON.parse(fs.readFileSync(abs, 'utf8')) as GraduationManifest;
  return parsed;
}

function hasScenarioPass(chaosRows: ScenarioRow[], adapterId: string, scenarioId: string): boolean {
  return chaosRows.some((row) => row.adapter === adapterId && row.scenario === scenarioId && row.ok);
}

function buildGraduationResults(
  manifest: GraduationManifest,
  baselineRows: ScenarioRow[],
  chaosRows: ScenarioRow[],
): AdapterGraduationResult[] {
  const manifestByAdapter = new Map(
    (manifest.adapters || []).map((row) => [cleanText(row.id || '', 80), row]),
  );
  const requiredScenarios = Array.isArray(manifest.required_scenarios) ? manifest.required_scenarios : [];
  const requiredHooks = Array.isArray(manifest.required_hooks) ? manifest.required_hooks : [];

  return ADAPTERS.map((adapter) => {
    const baselineRow = baselineRows.find((row) => row.adapter === adapter.id);
    const declared = manifestByAdapter.has(adapter.id);
    const scenarioPassCount = requiredScenarios.filter((id) => hasScenarioPass(chaosRows, adapter.id, id)).length;
    const missingScenarios = requiredScenarios.filter((id) => !hasScenarioPass(chaosRows, adapter.id, id));
    const receiptSchemaOk =
      Boolean(baselineRow?.detail) &&
      chaosRows
        .filter((row) => row.adapter === adapter.id)
        .every((row) => Boolean(row.expected_error) && Boolean(row.detail));
    const allFailClosed = requiredScenarios.every((scenarioId) => {
      const row = chaosRows.find((item) => item.adapter === adapter.id && item.scenario === scenarioId);
      return Boolean(row?.ok) && cleanText(row?.detail || '', 80).startsWith('fail_closed:');
    });

    const hookChecks = requiredHooks.map((hookId) => {
      if (hookId === 'health_check') {
        return {
          id: hookId,
          ok: baselineRow?.ok === true,
          detail: baselineRow?.ok === true ? 'baseline_ok' : cleanText(baselineRow?.detail || 'baseline_missing', 180),
        };
      }
      if (hookId === 'startup_timeout_policy') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'process_never_starts');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:process_never_starts' };
      }
      if (hookId === 'request_timeout_policy') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'starts_then_hangs');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:starts_then_hangs' };
      }
      if (hookId === 'fail_closed_policy_hooks') {
        return {
          id: hookId,
          ok: allFailClosed,
          detail: allFailClosed ? 'all_required_scenarios_fail_closed' : 'fail_closed_coverage_incomplete',
        };
      }
      if (hookId === 'receipt_schema_helpers') {
        return {
          id: hookId,
          ok: receiptSchemaOk,
          detail: receiptSchemaOk ? 'receipt_fields_present' : 'receipt_fields_missing',
        };
      }
      if (hookId === 'circuit_breaker_behavior') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'repeated_flapping');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:repeated_flapping' };
      }
      if (hookId === 'quarantine_hooks') {
        const ok = hasScenarioPass(chaosRows, adapter.id, 'repeated_flapping');
        return { id: hookId, ok, detail: ok ? 'scenario_pass' : 'missing_or_failed:repeated_flapping' };
      }
      return {
        id: hookId,
        ok: false,
        detail: 'unsupported_hook_id_in_manifest',
      };
    });

    const hookPassCount = hookChecks.filter((row) => row.ok).length;
    const hookTotal = hookChecks.length;
    const scenarioTotal = requiredScenarios.length;
    const graduated = declared && hookPassCount === hookTotal && scenarioPassCount === scenarioTotal;

    return {
      adapter: adapter.id,
      graduated,
      manifest_declared: declared,
      missing_scenarios: missingScenarios,
      hook_pass_count: hookPassCount,
      hook_total: hookTotal,
      hook_pass_ratio: hookTotal === 0 ? 0 : Number((hookPassCount / hookTotal).toFixed(4)),
      scenario_pass_count: scenarioPassCount,
      scenario_total: scenarioTotal,
      scenario_pass_ratio: scenarioTotal === 0 ? 0 : Number((scenarioPassCount / scenarioTotal).toFixed(4)),
      hooks: hookChecks,
    };
  });
}

function manifestConformanceViolations(manifest: GraduationManifest): string[] {
  const violations: string[] = [];
  const byId = new Map((manifest.adapters || []).map((row) => [cleanText(row.id || '', 80), row]));
  for (const adapter of ADAPTERS) {
    const declared = byId.get(adapter.id);
    if (!declared) {
      violations.push(`manifest_missing_adapter:${adapter.id}`);
      continue;
    }
    if (cleanText(declared.framework || '', 80) !== adapter.framework) {
      violations.push(`manifest_framework_mismatch:${adapter.id}`);
    }
    if (cleanText(declared.bridge_command || '', 120) !== adapter.bridgeCommand) {
      violations.push(`manifest_bridge_command_mismatch:${adapter.id}`);
    }
  }
  return violations;
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const root = process.cwd();
  const args = parseArgs(argv);
  if (!args.profile) {
    const payload = {
      ok: false,
      type: 'adapter_runtime_chaos_gate',
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

  let graduationManifest: GraduationManifest;
  try {
    graduationManifest = loadGraduationManifest(root, args.graduationManifestPath);
  } catch (error) {
    const payload = {
      ok: false,
      type: 'adapter_runtime_chaos_gate',
      error: 'adapter_graduation_manifest_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 500),
      manifest_path: args.graduationManifestPath,
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  let opsBinary: OpsBinary;
  try {
    opsBinary = resolveOpsBinary(root);
  } catch (error) {
    const payload = {
      ok: false,
      type: 'adapter_runtime_chaos_gate',
      error: 'adapter_runtime_chaos_binary_unavailable',
      detail: cleanText(error instanceof Error ? error.message : String(error), 500),
    };
    return emitStructuredResult(payload, {
      outPath: args.outPath,
      strict: args.strict,
      ok: false,
    });
  }

  const baselineRows: ScenarioRow[] = [];
  const chaosRows: ScenarioRow[] = [];
  const runtimeStateRoot = fs.mkdtempSync(path.join(os.tmpdir(), 'infring-adapter-chaos-'));

  for (const adapter of ADAPTERS) {
    const baseline = runBridgeCommand(
      root,
      opsBinary,
      adapter,
      baselinePayload(adapter),
      scenarioStatePath(runtimeStateRoot, adapter.id, 'baseline'),
    );
    const baselineOk = baseline.exitCode === 0 && baseline.payload?.ok === true;
    baselineRows.push({
      adapter: adapter.id,
      scenario: 'baseline',
      expected_error: '',
      ok: baselineOk,
      detail: baselineOk
        ? 'baseline_ok'
        : cleanText(
            `${baseline.payload?.error || 'missing_ok'};exit=${baseline.exitCode};stderr=${baseline.stderr};spawn_error=${
              baseline.spawnError || 'none'
            }`,
            280,
          ),
    });

    for (const chaosCase of CHAOS_CASES) {
      const chaosRun = runBridgeCommand(
        root,
        opsBinary,
        adapter,
        chaosPayload(adapter, chaosCase.id),
        scenarioStatePath(runtimeStateRoot, adapter.id, chaosCase.id),
      );
      const chaosError = cleanText(chaosRun.payload?.error || '', 120);
      const chaosOk =
        chaosRun.exitCode !== 0 &&
        chaosRun.payload?.ok === false &&
        chaosError.includes(chaosCase.expected_error);
      chaosRows.push({
        adapter: adapter.id,
        scenario: chaosCase.id,
        expected_error: chaosCase.expected_error,
        ok: chaosOk,
        detail: chaosOk
          ? `fail_closed:${chaosError}`
          : cleanText(
              `${chaosError || 'missing_error'};exit=${chaosRun.exitCode};stderr=${chaosRun.stderr};spawn_error=${
                chaosRun.spawnError || 'none'
              }`,
              280,
            ),
      });
    }
  }

  const baselinePassed = baselineRows.filter((row) => row.ok).length;
  const chaosPassed = chaosRows.filter((row) => row.ok).length;
  const allRows = baselineRows.concat(chaosRows);
  const baselineTotal = baselineRows.length;
  const chaosTotal = chaosRows.length;
  const baselinePassRatio = baselineTotal === 0 ? 0 : baselinePassed / baselineTotal;
  const chaosFailClosedRatio = chaosTotal === 0 ? 0 : chaosPassed / chaosTotal;

  const graduationResults = buildGraduationResults(graduationManifest, baselineRows, chaosRows);
  const graduationPassed = graduationResults.filter((row) => row.graduated).length;
  const graduationRatio = graduationResults.length === 0 ? 0 : graduationPassed / graduationResults.length;
  const manifestViolations = manifestConformanceViolations(graduationManifest);

  const failures = allRows
    .filter((row) => !row.ok)
    .map((row) => ({
      id: `${row.adapter}:${row.scenario}`,
      detail: row.detail,
    }))
    .concat(
      graduationResults
        .filter((row) => !row.graduated)
        .map((row) => ({
          id: `adapter_graduation_failed:${row.adapter}`,
          detail: `missing_scenarios=${row.missing_scenarios.join(',') || 'none'};hook_pass=${row.hook_pass_count}/${row.hook_total};manifest_declared=${String(row.manifest_declared)}`,
        })),
    )
    .concat(
      manifestViolations.map((detail) => ({
        id: 'adapter_graduation_manifest_violation',
        detail,
      })),
    );

  const metrics = {
    adapter_baseline_pass_ratio: Number(baselinePassRatio.toFixed(4)),
    adapter_chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
    adapter_chaos_scenarios_total: chaosTotal,
    adapter_graduation_ratio: Number(graduationRatio.toFixed(4)),
  };

  const report = {
    ok: failures.length === 0,
    type: 'adapter_runtime_chaos_gate',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    graduation_manifest_path: args.graduationManifestPath,
    summary: {
      adapters_total: ADAPTERS.length,
      baseline_total: baselineTotal,
      baseline_passed: baselinePassed,
      chaos_total: chaosTotal,
      chaos_fail_closed_passed: chaosPassed,
      chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
      graduation_passed: graduationPassed,
      graduation_ratio: Number(graduationRatio.toFixed(4)),
      manifest_violations: manifestViolations.length,
    },
    metrics,
    adapters: ADAPTERS.map((row) => ({
      id: row.id,
      bridge_command: row.bridgeCommand,
      framework: row.framework,
    })),
    baseline_results: baselineRows,
    chaos_results: chaosRows,
    graduation_results: graduationResults,
    graduation_policy: {
      version: graduationManifest.version,
      required_hooks: graduationManifest.required_hooks || [],
      required_scenarios: graduationManifest.required_scenarios || [],
      manifest_violations: manifestViolations,
    },
    failures,
  };

  if (cleanText(process.env.INFRING_ADAPTER_CHAOS_KEEP_TMP || '', 8) !== '1') {
    try {
      fs.rmSync(runtimeStateRoot, {
        recursive: true,
        force: true,
      });
    } catch {
      // best effort cleanup only
    }
  }

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
