#!/usr/bin/env node
'use strict';

import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const DEFAULT_OUT = path.join(process.cwd(), 'core/local/artifacts/release_verdict_current.json');
const DEFAULT_POLICY = path.join(process.cwd(), 'client/runtime/config/production_readiness_closure_policy.json');

function parseArgs(argv: string[]) {
  const parsed = parseStrictOutArgs(argv, { out: DEFAULT_OUT, strict: false });
  return {
    strict: parsed.strict,
    out: parsed.out || DEFAULT_OUT,
    policyPath: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY, 400),
    rootPath: cleanText(readFlag(argv, 'root') || '', 400),
  };
}

function resolveMaybe(root: string, maybePath: string): string {
  if (!maybePath) return '';
  if (path.isAbsolute(maybePath)) return maybePath;
  return path.resolve(root, maybePath);
}

function readJsonMaybe(filePath: string): any {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return null;
  }
}

function artifactOk(gateId: string, payload: any): boolean {
  switch (gateId) {
    case 'release_policy_gate':
      return payload?.ok === true;
    case 'ops:production-topology:gate':
      return (
        payload?.ok === true &&
        payload?.supported_production_topology === true &&
        Array.isArray(payload?.degraded_flags) &&
        payload.degraded_flags.length === 0
      );
    case 'chaos:continuous:gate':
    case 'state:kernel:replay':
      return payload?.ok === true;
    case 'ops:stateful-upgrade-rollback:gate':
    case 'ops:assimilation:v1:support:guard':
    case 'ops:release-blockers:gate':
    case 'ops:release-hardening-window:guard':
    case 'ops:release:scorecard:gate':
      return payload?.ok === true;
    case 'ops:orchestration:hidden-state:guard':
      return payload?.summary?.pass === true || payload?.summary?.violation_count === 0;
    case 'ops:production-closure:gate':
      return payload?.summary?.pass === true || payload?.ok === true;
    case 'ops:release:rc-rehearsal':
      return payload?.ok === true && payload?.summary?.candidate_ready === true;
    default:
      return payload?.ok === true;
  }
}

function artifactStrict(payload: any): boolean {
  return payload?.strict === true || payload?.inputs?.strict === true;
}

function fileDigest(filePath: string): string {
  return crypto.createHash('sha256').update(fs.readFileSync(filePath)).digest('hex');
}

export function buildReport(rawArgs = parseArgs(process.argv.slice(2))) {
  const args = typeof rawArgs === 'object' && rawArgs ? rawArgs : parseArgs(process.argv.slice(2));
  const root = path.resolve(args.rootPath || process.cwd());
  const policyPath = resolveMaybe(root, args.policyPath || DEFAULT_POLICY);
  const policy = readJsonMaybe(policyPath) || {};
  const verdictPolicy = policy.release_verdict || {};
  const requiredGateArtifacts = verdictPolicy.required_gate_artifacts || {};
  const checksumArtifactPaths = Array.isArray(verdictPolicy.checksum_artifact_paths)
    ? verdictPolicy.checksum_artifact_paths
    : [];
  const rcPath = resolveMaybe(root, requiredGateArtifacts['ops:release:rc-rehearsal'] || '');
  const rcPayload = readJsonMaybe(rcPath) || {};
  const rcSteps = Array.isArray(rcPayload?.steps) ? rcPayload.steps : [];
  const rcStepMap = new Map(rcSteps.map((row: any) => [String(row?.gate_id || ''), row]));
  const checks: Array<{ id: string; ok: boolean; detail: string }> = [
    {
      id: 'release_candidate_rehearsal_present',
      ok: fs.existsSync(rcPath),
      detail: rcPath ? path.relative(root, rcPath) : 'missing',
    },
    {
      id: 'release_candidate_rehearsal_strict',
      ok: artifactStrict(rcPayload),
      detail: `strict=${String(rcPayload?.strict === true || rcPayload?.inputs?.strict === true)}`,
    },
    {
      id: 'release_candidate_rehearsal_candidate_ready',
      ok: rcPayload?.summary?.candidate_ready === true && rcPayload?.summary?.required_steps_satisfied === true,
      detail: `candidate_ready=${String(rcPayload?.summary?.candidate_ready === true)};required_steps=${String(rcPayload?.summary?.required_steps_satisfied === true)}`,
    },
  ];

  for (const [gateId, relPath] of Object.entries(requiredGateArtifacts)) {
    const artifactPath = resolveMaybe(root, String(relPath || ''));
    const payload = readJsonMaybe(artifactPath);
    const step = rcStepMap.get(gateId);
    checks.push({
      id: `release_gate_step:${gateId}`,
      ok: gateId === 'ops:release:rc-rehearsal' ? rcPayload?.ok === true : step?.ok === true,
      detail:
        gateId === 'ops:release:rc-rehearsal'
          ? `present=${String(rcPayload?.ok === true)}`
          : `present=${String(Boolean(step))};ok=${String(step?.ok === true)}`,
    });
    checks.push({
      id: `release_gate_artifact:${gateId}`,
      ok: fs.existsSync(artifactPath),
      detail: artifactPath ? path.relative(root, artifactPath) : 'missing',
    });
    checks.push({
      id: `release_gate_health:${gateId}`,
      ok: artifactOk(gateId, payload),
      detail: `artifact_ok=${String(artifactOk(gateId, payload))}`,
    });
    if (gateId === 'release_policy_gate' || gateId === 'ops:release:scorecard:gate' || gateId === 'ops:production-closure:gate') {
      checks.push({
        id: `release_gate_strict_artifact:${gateId}`,
        ok: artifactStrict(payload),
        detail: `strict=${String(artifactStrict(payload))}`,
      });
    }
  }

  const artifact_hashes = checksumArtifactPaths.map((relPath: string) => {
    const artifactPath = resolveMaybe(root, relPath);
    const exists = fs.existsSync(artifactPath);
    return {
      path: relPath,
      exists,
      sha256: exists ? fileDigest(artifactPath) : '',
    };
  });
  const verdict_checksum = crypto
    .createHash('sha256')
    .update(
      artifact_hashes
        .map((row) => `${row.path}:${row.exists ? row.sha256 : 'missing'}`)
        .join('\n'),
    )
    .digest('hex');
  const failed = checks.filter((row) => !row.ok);
  return {
    root,
    outPath: resolveMaybe(root, args.out || DEFAULT_OUT),
    report: {
      ok: failed.length === 0,
      type: 'release_verdict',
      generated_at: new Date().toISOString(),
      strict: Boolean(args.strict),
      summary: {
        check_count: checks.length,
        failed_count: failed.length,
        release_ready: failed.length === 0,
      },
      failed_ids: failed.map((row) => row.id),
      checks,
      artifact_hashes,
      verdict_checksum,
    },
  };
}

export function run(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const result = buildReport(args);
  return emitStructuredResult(result.report, {
    outPath: result.outPath,
    strict: args.strict,
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
