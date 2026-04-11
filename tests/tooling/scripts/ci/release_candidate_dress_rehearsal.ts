#!/usr/bin/env node
'use strict';

import { parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';
import { DEFAULT_GATE_REGISTRY_PATH, executeGate } from '../../lib/runner.ts';
import path from 'node:path';

const DEFAULT_OUT = path.join(
  process.cwd(),
  'core/local/artifacts/release_candidate_dress_rehearsal_current.json',
);

const DEFAULT_SEQUENCE = [
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday',
  'dr:gameday:gate',
  'ops:legacy-runner:release-guard',
  'ops:production-topology:gate',
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

function buildReport(argv: string[] = process.argv.slice(2)) {
  const args = readRehearsalArgs(argv);
  const previousHardeningValue = process.env.INFRING_RELEASE_HARDENING_WINDOW;
  if (args.activateHardening) process.env.INFRING_RELEASE_HARDENING_WINDOW = '1';
  try {
    const steps = DEFAULT_SEQUENCE.map((gateId, index) => {
      const report = executeGate(gateId, {
        registryPath: DEFAULT_GATE_REGISTRY_PATH,
        strict: true,
      });
      return {
        order: index + 1,
        gate_id: gateId,
        ok: report.ok,
        duration_ms: report.duration_ms,
        exit_code: report.summary.exit_code,
        artifact_paths: report.artifact_paths,
        failure: report.failures[0]?.detail || '',
      };
    });
    const failed = steps.filter((row) => !row.ok);
    return {
      ok: failed.length === 0,
      type: 'release_candidate_dress_rehearsal',
      generated_at: new Date().toISOString(),
      inputs: {
        activate_hardening_window: args.activateHardening,
        registry_path: DEFAULT_GATE_REGISTRY_PATH,
      },
      summary: {
        step_count: steps.length,
        failed_count: failed.length,
      },
      failures: failed,
      artifact_paths: steps.flatMap((row) => row.artifact_paths || []),
      steps,
    };
  } finally {
    if (args.activateHardening) {
      if (previousHardeningValue == null) delete process.env.INFRING_RELEASE_HARDENING_WINDOW;
      else process.env.INFRING_RELEASE_HARDENING_WINDOW = previousHardeningValue;
    }
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
