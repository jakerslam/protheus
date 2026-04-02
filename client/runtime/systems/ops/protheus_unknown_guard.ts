#!/usr/bin/env node
'use strict';

const { runProtheusOps } = require('./run_protheus_ops.ts');

function isJsonMode(argv: string[]): boolean {
  return argv.some((arg) => arg === '--json' || arg === '--json=1');
}

function firstUnknownCommand(argv: string[]): string {
  for (const raw of argv) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token === '--json' || token === '--json=1') continue;
    if (token === '--help' || token === '-h') continue;
    if (token.startsWith('-')) continue;
    return token;
  }
  return '';
}

function run(argv: string[] = process.argv.slice(2)): number {
  const tokens = Array.isArray(argv)
    ? argv.map((token) => String(token || '').trim()).filter(Boolean)
    : [];
  const unknown = firstUnknownCommand(tokens);
  const json = isJsonMode(tokens);
  if (json) {
    process.stdout.write(
      `${JSON.stringify({
        ok: false,
        type: 'protheus_unknown_guard',
        error: 'unknown_command',
        command: unknown,
        hint: 'Run `infring help` to list available commands.',
      })}\n`,
    );
    return 2;
  }
  if (unknown) {
    process.stderr.write(`[infring] unknown command: ${unknown}\n`);
  } else {
    process.stderr.write('[infring] unknown command\n');
  }
  runProtheusOps(['command-list-kernel', '--mode=help'], {
    unknownDomainFallback: false,
  });
  return 2;
}

if (require.main === module) {
  process.exit(run(process.argv.slice(2)));
}

module.exports = { run };
