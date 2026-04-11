#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { invokeTsModuleSync } from '../../../../client/runtime/lib/in_process_ts_delegate.ts';

type CommandResult = {
  id: string;
  ok: boolean;
  status: number;
  duration_ms: number;
  command: string;
  args: string[];
  stdout: string;
  stderr: string;
  payload: unknown;
};

type ParsedArgs = {
  command: string;
  out: string;
  strict: boolean;
};

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const DEFAULT_OUT = path.join(ROOT, 'core/local/artifacts/support_bundle_latest.json');
const CLOSURE_POLICY_PATH = path.join(ROOT, 'client/runtime/config/production_readiness_closure_policy.json');
const ASSIMILATION_V1_PATH = path.join(ROOT, 'client/runtime/config/assimilation_v1_support_contract.json');
const BLOCKER_RUBRIC_PATH = path.join(ROOT, 'client/runtime/config/release_blocker_rubric.json');
const HARDENING_WINDOW_PATH = path.join(ROOT, 'client/runtime/config/release_hardening_window_policy.json');
const RELEASE_SCORECARD_PATH = path.join(
  ROOT,
  'client/runtime/local/state/release/scorecard/release_scorecard.json',
);

function clean(value: unknown, max = 500): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseBool(value: string | undefined, fallback = false): boolean {
  const raw = clean(value, 32).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function parseArgs(argv: string[]): ParsedArgs {
  const parsed: ParsedArgs = {
    command: 'run',
    out: DEFAULT_OUT,
    strict: false,
  };
  for (const tokenRaw of argv) {
    const token = clean(tokenRaw, 500);
    if (!token) continue;
    if (token === 'run' || token === 'status' || token === 'help') {
      parsed.command = token;
      continue;
    }
    if (token.startsWith('--out=')) {
      parsed.out = path.resolve(ROOT, clean(token.slice('--out='.length), 500));
      continue;
    }
    if (token.startsWith('--strict=')) {
      parsed.strict = parseBool(token.slice('--strict='.length), false);
      continue;
    }
  }
  return parsed;
}

function parseJsonLine(stdout: string): unknown {
  const whole = String(stdout || '').trim();
  if (whole) {
    try {
      return JSON.parse(whole);
    } catch {}
  }
  const lines = String(stdout || '')
    .split('\n')
    .map((row) => row.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function readJsonMaybe(filePath: string): unknown {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function runTsCommand(id: string, scriptRelPath: string, args: string[] = []): CommandResult {
  const started = Date.now();
  const scriptAbs = path.join(ROOT, scriptRelPath);
  const out = invokeTsModuleSync(scriptAbs, {
    argv: args,
    cwd: ROOT,
    exportName: 'run',
    teeStdout: false,
    teeStderr: false,
  });
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  const stdout = String(out.stdout || '');
  const stderr = String(out.stderr || '');
  return {
    id,
    ok: status === 0,
    status,
    duration_ms: Date.now() - started,
    command: 'in_process_ts_delegate',
    args: [scriptRelPath].concat(args),
    stdout,
    stderr,
    payload: parseJsonLine(stdout),
  };
}

function checkFile(pathRel: string) {
  const abs = path.join(ROOT, pathRel);
  return {
    path: pathRel,
    exists: fs.existsSync(abs),
  };
}

function buildBundle(outPath: string) {
  const checks: CommandResult[] = [
    runTsCommand('transport_topology', 'client/runtime/systems/ops/transport_topology_status.ts', [
      '--json=1',
    ]),
    runTsCommand(
      'legacy_process_runner_release_guard',
      'tests/tooling/scripts/ci/legacy_process_runner_release_guard.ts',
      ['--strict=0', '--out=core/local/artifacts/legacy_process_runner_release_guard_current.json'],
    ),
    runTsCommand('production_topology_diagnostic', 'tests/tooling/scripts/ops/production_topology_diagnostic.ts', [
      '--out=core/local/artifacts/production_topology_diagnostic_current.json',
    ]),
    runTsCommand('transport_spawn_audit', 'tests/tooling/scripts/ci/transport_spawn_audit.ts', [
      '--strict=0',
      '--out=core/local/artifacts/transport_spawn_audit_current.json',
    ]),
    runTsCommand('arch_boundary_conformance', 'tests/tooling/scripts/ci/arch_boundary_conformance.ts', [
      '--strict=0',
      '--out=core/local/artifacts/arch_boundary_conformance_current.json',
    ]),
    runTsCommand('production_closure', 'tests/tooling/scripts/ci/production_readiness_closure_gate.ts', [
      '--strict=0',
      '--run-smoke=0',
      '--out=core/local/artifacts/production_readiness_closure_gate_current.json',
    ]),
    runTsCommand('release_policy_gate', 'tests/tooling/scripts/ci/release_policy_gate.ts', [
      '--strict=0',
      '--out=core/local/artifacts/release_policy_gate_current.json',
    ]),
    runTsCommand('stateful_upgrade_rollback', 'tests/tooling/scripts/ci/stateful_upgrade_rollback_gate.ts', [
      '--strict=0',
      '--out=core/local/artifacts/stateful_upgrade_rollback_gate_current.json',
    ]),
    runTsCommand('assimilation_v1_support_guard', 'tests/tooling/scripts/ci/assimilation_v1_support_guard.ts', [
      '--strict=0',
      '--out=core/local/artifacts/assimilation_v1_support_guard_current.json',
    ]),
    runTsCommand('release_blocker_rubric', 'tests/tooling/scripts/ci/release_blocker_rubric_gate.ts', [
      '--strict=0',
      '--out=core/local/artifacts/release_blocker_rubric_current.json',
    ]),
    runTsCommand('release_hardening_window', 'tests/tooling/scripts/ci/release_hardening_window_guard.ts', [
      '--strict=0',
      '--out=core/local/artifacts/release_hardening_window_guard_current.json',
    ]),
    runTsCommand('recovery_rehearsal', 'tests/tooling/scripts/ops/dr_gameday.ts', [
      'gate',
    ]),
    runTsCommand('runtime_telemetry_status', 'tests/tooling/scripts/ops/runtime_telemetry_optin.ts', [
      'status',
    ]),
  ];

  const files = [
    checkFile('core/local/artifacts/release_policy_gate_current.json'),
    checkFile('core/local/artifacts/production_readiness_closure_gate_current.json'),
    checkFile('core/local/artifacts/production_topology_diagnostic_current.json'),
    checkFile('core/local/artifacts/transport_spawn_audit_current.json'),
    checkFile('core/local/artifacts/legacy_process_runner_release_guard_current.json'),
    checkFile('core/local/artifacts/stateful_upgrade_rollback_gate_current.json'),
    checkFile('core/local/artifacts/assimilation_v1_support_guard_current.json'),
    checkFile('core/local/artifacts/release_blocker_rubric_current.json'),
    checkFile('core/local/artifacts/release_hardening_window_guard_current.json'),
    checkFile('core/local/artifacts/arch_boundary_conformance_current.json'),
    checkFile('client/runtime/config/production_readiness_closure_policy.json'),
    checkFile('client/runtime/config/assimilation_v1_support_contract.json'),
    checkFile('client/runtime/config/release_blocker_rubric.json'),
    checkFile('client/runtime/config/release_hardening_window_policy.json'),
    checkFile('client/runtime/local/state/release/scorecard/release_scorecard.json'),
  ];
  const payloadChecks = checks.filter((row) => row.payload && typeof row.payload === 'object');
  const degradedFlags = checks
    .flatMap((row) =>
      Array.isArray((row.payload as any)?.degraded_flags) ? (row.payload as any).degraded_flags : [],
    )
    .filter(Boolean);
  const maxCommandLatencyMs = checks.reduce((max, row) => Math.max(max, Number(row.duration_ms || 0)), 0);
  const receiptCompletenessRate = checks.length === 0 ? 1 : payloadChecks.length / checks.length;

  const report = {
    ok: checks.every((row) => row.ok),
    type: 'support_bundle',
    generated_at: new Date().toISOString(),
    host: {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
      cwd: ROOT,
      hostname: os.hostname(),
    },
    metrics: {
      total_checks: checks.length,
      receipted_checks: payloadChecks.length,
      receipt_completeness_rate: Number(receiptCompletenessRate.toFixed(4)),
      max_command_latency_ms: maxCommandLatencyMs,
    },
    closure_evidence: {
      transport_topology: checks.find((row) => row.id === 'transport_topology')?.payload || null,
      legacy_process_runner_release_guard:
        checks.find((row) => row.id === 'legacy_process_runner_release_guard')?.payload || null,
      production_topology_diagnostic:
        checks.find((row) => row.id === 'production_topology_diagnostic')?.payload || null,
      transport_spawn_audit:
        checks.find((row) => row.id === 'transport_spawn_audit')?.payload || null,
      arch_boundary_conformance:
        checks.find((row) => row.id === 'arch_boundary_conformance')?.payload || null,
      production_closure: checks.find((row) => row.id === 'production_closure')?.payload || null,
      release_policy_gate: checks.find((row) => row.id === 'release_policy_gate')?.payload || null,
      stateful_upgrade_rollback:
        checks.find((row) => row.id === 'stateful_upgrade_rollback')?.payload || null,
      assimilation_v1_support_guard:
        checks.find((row) => row.id === 'assimilation_v1_support_guard')?.payload || null,
      release_blocker_rubric:
        checks.find((row) => row.id === 'release_blocker_rubric')?.payload || null,
      release_hardening_window:
        checks.find((row) => row.id === 'release_hardening_window')?.payload || null,
      recovery_rehearsal: checks.find((row) => row.id === 'recovery_rehearsal')?.payload || null,
      runtime_telemetry_status:
        checks.find((row) => row.id === 'runtime_telemetry_status')?.payload || null,
    },
    closure_contracts: {
      production_readiness_closure_policy: readJsonMaybe(CLOSURE_POLICY_PATH),
      assimilation_v1_support_contract: readJsonMaybe(ASSIMILATION_V1_PATH),
      release_blocker_rubric: readJsonMaybe(BLOCKER_RUBRIC_PATH),
      release_hardening_window_policy: readJsonMaybe(HARDENING_WINDOW_PATH),
    },
    closure_artifacts: {
      release_scorecard: readJsonMaybe(RELEASE_SCORECARD_PATH),
    },
    degraded_flags: degradedFlags,
    checks,
    files,
  };

  fs.mkdirSync(path.dirname(outPath), { recursive: true });
  fs.writeFileSync(outPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
  return report;
}

function run(argv: string[] = process.argv.slice(2)): number {
  const parsed = parseArgs(argv);
  if (parsed.command === 'help') {
    console.log('Usage: ops:support-bundle:export [run|status] [--out=<path>] [--strict=1|0]');
    return 0;
  }
  const outPath = parsed.out || DEFAULT_OUT;
  if (parsed.command === 'status') {
    if (!fs.existsSync(outPath)) {
      console.log(
        JSON.stringify({
          ok: false,
          type: 'support_bundle_status',
          error: 'support_bundle_missing',
          out: outPath,
        }),
      );
      return parsed.strict ? 1 : 0;
    }
    const payload = JSON.parse(fs.readFileSync(outPath, 'utf8'));
    console.log(JSON.stringify({ ok: true, type: 'support_bundle_status', out: outPath, payload }));
    return parsed.strict && payload.ok !== true ? 1 : 0;
  }
  const bundle = buildBundle(outPath);
  console.log(JSON.stringify({ ok: bundle.ok, type: 'support_bundle_run', out: outPath, bundle }));
  if (parsed.strict && bundle.ok !== true) return 1;
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = {
  run,
  buildBundle,
};
