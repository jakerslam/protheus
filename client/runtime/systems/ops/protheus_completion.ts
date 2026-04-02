#!/usr/bin/env node
'use strict';

const CORE_COMMANDS = [
  'gateway',
  'start',
  'stop',
  'restart',
  'status',
  'dashboard',
  'doctor',
  'verify-install',
  'dream',
  'compact',
  'proactive_daemon',
  'speculate',
  'setup',
  'help',
];

function jsonMode(argv: string[]): boolean {
  return argv.some((arg) => arg === '--json' || arg === '--json=1');
}

function run(argv: string[] = process.argv.slice(2)): number {
  if (argv.includes('--help') || argv.includes('-h')) {
    process.stdout.write(
      'Usage: infring completion [--json]\n' +
        'Prints completion candidates for core entry commands.\n',
    );
    return 0;
  }
  if (jsonMode(argv)) {
    process.stdout.write(
      `${JSON.stringify({ ok: true, type: 'protheus_completion', commands: CORE_COMMANDS })}\n`,
    );
    return 0;
  }
  process.stdout.write(`${CORE_COMMANDS.join('\n')}\n`);
  return 0;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
