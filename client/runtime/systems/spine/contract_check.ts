#!/usr/bin/env tsx
// Compatibility shim: routes legacy merge-guard contract-check entrypoint into Rust authority.

import { runProtheusOps } from '../ops/run_protheus_ops.js';

export function run(argv: string[] = process.argv.slice(2)): number {
  const args = argv.length ? ['contract-check', ...argv] : ['contract-check', 'status'];
  return runProtheusOps(args, { unknownDomainFallback: false });
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}
