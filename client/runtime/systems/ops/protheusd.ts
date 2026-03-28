#!/usr/bin/env tsx
// Thin strict conduit launcher. Tokens in this file are audited by rust_source_of_truth policy.

import { createOpsLaneBridge } from '../../lib/rust_lane_bridge.ts';
import { runProtheusOps } from './run_protheus_ops.ts';

const PROTHEUS_CONDUIT_STRICT = process.env.PROTHEUS_CONDUIT_STRICT ?? '1';
const gateBridge = createOpsLaneBridge(__dirname, 'protheusd', 'protheusd-launcher-kernel', {
  preferLocalCore: true,
});

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

export function run(argv: string[] = process.argv.slice(2)): number {
  const gatePayload = Buffer.from(JSON.stringify({ argv }), 'utf8').toString('base64');
  const gateOut = gateBridge.run(['gate', `--payload-base64=${gatePayload}`]);
  const gate = (gateOut && typeof gateOut.payload === 'object' && gateOut.payload)
    ? (gateOut.payload as Record<string, unknown>)
    : (parseLastJson(gateOut?.stdout || '') || {});
  const gateOk = gate?.ok === true;
  const passArgs = Array.isArray(gate?.pass_args)
    ? gate.pass_args.map((item) => String(item || ''))
    : argv;
  const gateExitCode = Number.isFinite(Number(gate?.exit_code)) ? Number(gate?.exit_code) : 1;
  const strict = PROTHEUS_CONDUIT_STRICT !== '0';
  const legacyConduitMissing = process.env.PROTHEUS_CONDUIT_AVAILABLE === '0';
  const legacyAllowFallback = hasFlag(argv, '--allow-legacy-fallback');

  // If kernel routing is unavailable, preserve previous fail-closed behavior.
  if (!gateOk && strict && legacyConduitMissing && !legacyAllowFallback) {
    console.error('conduit_required_strict');
    return 2;
  }
  if (!gateOk) return gateExitCode;

  return runProtheusOps(passArgs, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
