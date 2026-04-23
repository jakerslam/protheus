#!/usr/bin/env tsx
// Thin strict conduit launcher. Tokens in this file are audited by rust_source_of_truth policy.

import { createOpsLaneBridge } from '../../lib/rust_lane_bridge.ts';
import { runInfringOps } from './run_infring_ops.ts';

const INFRING_CONDUIT_STRICT = process.env.INFRING_CONDUIT_STRICT ?? '1';
const gateBridge = createOpsLaneBridge(__dirname, 'infringd', 'infringd-launcher-kernel', {
  preferLocalCore: true,
});

function normalizeArgs(argv: string[] = process.argv.slice(2)): string[] {
  return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : [];
}

function parseLastJson(stdout: string): Record<string, unknown> | null {
  const lines = String(stdout || '')
    .trim()
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {
      // keep scanning
    }
  }
  return null;
}

function hasFlag(argv: string[], flag: string): boolean {
  return argv.includes(flag);
}

function resolveGatePayload(gateOut: { payload?: unknown; stdout?: string }): Record<string, unknown> {
  if (gateOut && typeof gateOut.payload === 'object' && gateOut.payload) {
    return gateOut.payload as Record<string, unknown>;
  }
  return parseLastJson(gateOut?.stdout || '') || {};
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = normalizeArgs(argv);
  const gatePayload = Buffer.from(JSON.stringify({ argv: args }), 'utf8').toString('base64');
  const gateOut = gateBridge.run(['gate', `--payload-base64=${gatePayload}`]);
  const gate = resolveGatePayload(gateOut);
  const gateOk = gate?.ok === true;
  const passArgs = Array.isArray(gate?.pass_args)
    ? gate.pass_args.map((item) => String(item || ''))
    : args;
  const gateExitCode = Number.isFinite(Number(gate?.exit_code)) ? Number(gate?.exit_code) : 1;
  const strict = INFRING_CONDUIT_STRICT !== '0';
  const legacyConduitMissing = process.env.INFRING_CONDUIT_AVAILABLE === '0';
  const legacyAllowFallback = hasFlag(args, '--allow-legacy-fallback');

  // If kernel routing is unavailable, preserve previous fail-closed behavior.
  if (!gateOk && strict && legacyConduitMissing && !legacyAllowFallback) {
    console.error('conduit_required_strict');
    return 2;
  }
  if (!gateOk && gate && typeof gate.error === 'string' && gate.error.trim()) {
    console.error(gate.error);
  }
  if (!gateOk) return gateExitCode;

  return runInfringOps(passArgs, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
