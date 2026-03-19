#!/usr/bin/env tsx
// Compatibility lane for `protheus-ops status --dashboard`.

import { runProtheusOps } from './run_protheus_ops.js';

export function run(argv: string[] = process.argv.slice(2)): number {
  const passthrough = argv.length ? argv : ['status', '--dashboard'];
  return runProtheusOps(passthrough, { unknownDomainFallback: true });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
