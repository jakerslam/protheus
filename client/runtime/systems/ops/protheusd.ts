#!/usr/bin/env tsx
// Thin strict conduit launcher. Tokens in this file are audited by rust_source_of_truth policy.

import { runProtheusOps } from './run_protheus_ops.ts';

const PROTHEUS_CONDUIT_STRICT = process.env.PROTHEUS_CONDUIT_STRICT ?? '1';

function hasFlag(argv: string[], flag: string): boolean {
  return argv.includes(flag);
}

export function run(argv: string[] = process.argv.slice(2)): number {
  const allowLegacyFallback = hasFlag(argv, '--allow-legacy-fallback');
  const strict = PROTHEUS_CONDUIT_STRICT !== '0';
  const conduitMissing = process.env.PROTHEUS_CONDUIT_AVAILABLE === '0';

  if (strict && conduitMissing && !allowLegacyFallback) {
    console.error('conduit_required_strict');
    return 2;
  }

  return runProtheusOps(argv, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
