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
  const failures = allRows.filter((row) => !row.ok).map((row) => ({
    id: `${row.adapter}:${row.scenario}`,
    detail: row.detail,
  }));
  const baselineTotal = baselineRows.length;
  const chaosTotal = chaosRows.length;
  const baselinePassRatio = baselineTotal === 0 ? 0 : baselinePassed / baselineTotal;
  const chaosFailClosedRatio = chaosTotal === 0 ? 0 : chaosPassed / chaosTotal;
  const metrics = {
    adapter_baseline_pass_ratio: Number(baselinePassRatio.toFixed(4)),
    adapter_chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
    adapter_chaos_scenarios_total: chaosTotal,
  };

  const report = {
    ok: failures.length === 0,
    type: 'adapter_runtime_chaos_gate',
    profile: args.profile,
    generated_at: new Date().toISOString(),
    revision: currentRevision(root),
    summary: {
      adapters_total: ADAPTERS.length,
      baseline_total: baselineTotal,
      baseline_passed: baselinePassed,
      chaos_total: chaosTotal,
      chaos_fail_closed_passed: chaosPassed,
      chaos_fail_closed_ratio: Number(chaosFailClosedRatio.toFixed(4)),
    },
    metrics,
    adapters: ADAPTERS.map((row) => ({
      id: row.id,
      bridge_command: row.bridgeCommand,
      framework: row.framework,
    })),
    baseline_results: baselineRows,
    chaos_results: chaosRows,
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
