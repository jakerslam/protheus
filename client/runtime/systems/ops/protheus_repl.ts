#!/usr/bin/env node
'use strict';

const { runProtheusOps } = require('./run_protheus_ops.ts');

function run(argv: string[] = process.argv.slice(2)): number {
  if (argv.includes('--help') || argv.includes('-h')) {
    process.stdout.write(
      'Usage: infring repl\n' +
        'Lightweight REPL bootstrap for constrained installs.\n',
    );
    return 0;
  }
  // Minimal interactive fallback that always lands on a valid, core-owned command surface.
  const status = runProtheusOps(['command-list-kernel', '--mode=help'], {
    unknownDomainFallback: false,
  });
  if (status === 0 && process.stdin.isTTY) {
    process.stdout.write(
      '[infring repl] interactive shell is unavailable in slim runtime; showing command index.\n',
    );
  }
  return status;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
