#!/usr/bin/env node
'use strict';

import { parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { DEFAULT_GATE_REGISTRY_PATH, executeGate } from '../../lib/runner.ts';
import fs from 'node:fs';
import path from 'node:path';

const DEFAULT_OUT = path.join(
  process.cwd(),
  'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
);
const CLOSURE_POLICY_PATH = path.join(
  process.cwd(),
  'client/runtime/config/production_readiness_closure_policy.json',
);

const DEFAULT_SEQUENCE = [
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday:gate',
  'release_policy_gate',
  'ops:legacy-runner:release-guard',
  'ops:production-topology:gate',
  'audit:client-layer-boundary',
  'ops:stateful-upgrade-rollback:gate',
  'ops:assimilation:v1:support:guard',
  'ops:release-blockers:gate',
  'ops:release-hardening-window:guard',
  'ops:support-bundle:export',
  'ops:release:scorecard:gate',
  'ops:production-closure:gate',
];

function readRehearsalArgs(argv: string[]) {
  const parsed = parseStrictOutArgs(argv, { out: DEFAULT_OUT, strict: false });
  const activateHardening = parseBool(readFlag(argv, 'activate-hardening'), true);
  return {
    strict: parsed.strict,
    out: parsed.out || DEFAULT_OUT,
    activateHardening,
  };
}

function clean(value: unknown, max = 240): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function parseJsonPayload(raw: string): any {
  const whole = String(raw || '').trim();
  if (!whole) return null;
  try {
    return JSON.parse(whole);
  } catch {
    return null;
  }
}

function readRequiredStepIds(): string[] {
  try {
    const policy = JSON.parse(fs.readFileSync(CLOSURE_POLICY_PATH, 'utf8'));
    const configured = policy?.release_candidate_rehearsal?.required_step_gate_ids;
    return Array.isArray(configured) ? configured.map((row: unknown) => clean(row, 160)).filter(Boolean) : [];
  } catch {
    return [];
  }
}

function buildReport(argv: string[] = process.argv.slice(2)) {
  const args = readRehearsalArgs(argv);
  const requiredStepGateIds = readRequiredStepIds();
  const previousHardeningValue = process.env.INFRING_RELEASE_HARDENING_WINDOW;
  const previousRcActiveValue = process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE;
  if (args.activateHardening) process.env.INFRING_RELEASE_HARDENING_WINDOW = '1';
  process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE = '1';
  try {
    const steps = DEFAULT_SEQUENCE.map((gateId, index) => {
      const report = executeGate(gateId, {
        registryPath: DEFAULT_GATE_REGISTRY_PATH,
        strict: true,
      });
      const payload = parseJsonPayload(report.stdout);
      return {
        order: index + 1,
        gate_id: gateId,
        ok: report.ok,
        duration_ms: report.duration_ms,
        exit_code: report.summary.exit_code,
        artifact_paths: report.artifact_paths,
        failure: report.failures[0]?.detail || '',
        payload_type: clean(payload?.type || '', 120),
        gate_state: clean(payload?.gate_state || '', 120),
        failed_ids: Array.isArray(payload?.failed_ids) ? payload.failed_ids : [],
        degraded_flags: Array.isArray(payload?.degraded_flags) ? payload.degraded_flags : [],
        payload_summary: payload?.summary || null,
      };
    });
    const failed = steps.filter((row) => !row.ok);
    const passedGateIds = new Set(steps.filter((row) => row.ok).map((row) => row.gate_id));
    const requiredStepsSatisfied =
      requiredStepGateIds.length === 0 || requiredStepGateIds.every((gateId) => passedGateIds.has(gateId));
    const recoveryStep = steps.find((row) => row.gate_id === 'dr:gameday:gate');
    const topologyStep = steps.find((row) => row.gate_id === 'ops:production-topology:gate');
    const clientBoundaryStep = steps.find((row) => row.gate_id === 'audit:client-layer-boundary');
    return {
      ok: failed.length === 0 && requiredStepsSatisfied,
      type: 'release_candidate_dress_rehearsal',
      generated_at: new Date().toISOString(),
      strict: args.strict,
      inputs: {
        activate_hardening_window: args.activateHardening,
        registry_path: DEFAULT_GATE_REGISTRY_PATH,
        required_step_gate_ids: requiredStepGateIds,
      },
      summary: {
        step_count: steps.length,
        failed_count: failed.length,
        required_step_count: requiredStepGateIds.length,
        required_steps_satisfied: requiredStepsSatisfied,
        candidate_ready: failed.length === 0 && requiredStepsSatisfied,
      },
      failures: failed,
      artifact_paths: steps.flatMap((row) => row.artifact_paths || []),
      recovery_rehearsal: {
        gate_state: clean(recoveryStep?.gate_state || '', 120),
        ok: recoveryStep?.ok === true,
      },
      topology: {
        ok: topologyStep?.ok === true,
        degraded_flags: topologyStep?.degraded_flags || [],
      },
      client_boundary: {
        ok: clientBoundaryStep?.ok === true,
        failed_ids: clientBoundaryStep?.failed_ids || [],
      },
      steps,
    };
  } finally {
    if (args.activateHardening) {
      if (previousHardeningValue == null) delete process.env.INFRING_RELEASE_HARDENING_WINDOW;
      else process.env.INFRING_RELEASE_HARDENING_WINDOW = previousHardeningValue;
    }
    if (previousRcActiveValue == null) delete process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE;
    else process.env.INFRING_RELEASE_RC_REHEARSAL_ACTIVE = previousRcActiveValue;
  }
}

function run(argv: string[] = process.argv.slice(2)) {
  const args = readRehearsalArgs(argv);
  const report = buildReport(argv);
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
