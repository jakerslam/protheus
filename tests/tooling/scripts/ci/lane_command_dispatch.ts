#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, parseBool, readFlag } from '../../lib/cli.ts';
import { runCommand } from '../../lib/process.ts';

const ROOT = process.cwd();

type Entry = {
  id: string;
  command: string;
  source_script?: string;
};

type Registry = {
  version: string;
  run?: Record<string, Entry>;
  test?: Record<string, Entry>;
};

function parseArgs(argv: string[]) {
  const mode = cleanText(argv[0] || 'list', 32).toLowerCase();
  const out = {
    mode: ['run', 'test', 'list'].includes(mode) ? mode : 'list',
    registry: cleanText(readFlag(argv.slice(1), 'registry') || 'client/runtime/config/lane_command_registry.json', 260),
    id: cleanText(readFlag(argv.slice(1), 'id') || '', 120).toUpperCase(),
    listMode: cleanText(readFlag(argv.slice(1), 'mode') || 'all', 16).toLowerCase(),
    json: argv.slice(1).includes('--json') || parseBool(readFlag(argv.slice(1), 'json'), false),
  };
  if (!['run', 'test', 'all'].includes(out.listMode)) out.listMode = 'all';
  return out;
}

function readRegistry(registryPath: string): Registry {
  return JSON.parse(fs.readFileSync(path.resolve(ROOT, registryPath), 'utf8')) as Registry;
}

function listEntries(registry: Registry, mode: 'run' | 'test' | 'all') {
  const sections = mode === 'all' ? (['run', 'test'] as const) : ([mode] as const);
  return sections.flatMap((section) =>
    Object.entries(registry[section] || {})
      .map(([id, entry]) => ({
        section,
        id,
        command: cleanText(entry.command, 800),
        source_script: cleanText(entry.source_script || '', 160) || null,
      }))
      .sort((left, right) => left.id.localeCompare(right.id)),
  );
}

function executeCommand(command: string) {
  const child = runCommand([command], {
    cwd: ROOT,
    env: process.env,
    shell: true,
    inheritStdio: true,
    timeoutSec: 3600,
  });
  return {
    ok: child.ok,
    exit_code: child.status,
    signal: child.signal ?? null,
  };
}

function main(): number {
  const args = parseArgs(process.argv.slice(2));
  const registry = readRegistry(args.registry);

  if (args.mode === 'list') {
    const entries = listEntries(registry, args.listMode as 'run' | 'test' | 'all');
    const payload = {
      ok: true,
      type: 'lane_command_dispatch_list',
      registry_path: cleanText(args.registry, 260),
      mode: args.listMode,
      count: entries.length,
      entries,
    };
    if (args.json) {
      process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
      return 0;
    }
    const lines = [
      'Lane Command Registry',
      '',
      `Mode: ${args.listMode}`,
      `Entries: ${entries.length}`,
      '',
      ...entries.slice(0, 200).map(
        (entry) => `- [${entry.section}] ${entry.id} -> ${entry.command}`,
      ),
    ];
    process.stdout.write(`${lines.join('\n')}\n`);
    return 0;
  }

  const section = args.mode as 'run' | 'test';
  const id = cleanText(args.id, 120).toUpperCase();
  if (!id) {
    process.stderr.write(
      `${JSON.stringify({ ok: false, type: 'lane_command_dispatch', error: 'missing_id', section }, null, 2)}\n`,
    );
    return 1;
  }

  const entry = registry[section]?.[id];
  if (!entry || !cleanText(entry.command, 800)) {
    process.stderr.write(
      `${JSON.stringify(
        {
          ok: false,
          type: 'lane_command_dispatch',
          error: 'id_not_registered',
          section,
          id,
          registry_path: cleanText(args.registry, 260),
        },
        null,
        2,
      )}\n`,
    );
    return 1;
  }

  const result = executeCommand(entry.command);
  if (args.json) {
    process.stdout.write(
      `${JSON.stringify(
        {
          ok: result.ok,
          type: 'lane_command_dispatch',
          section,
          id,
          command: cleanText(entry.command, 800),
          source_script: cleanText(entry.source_script || '', 160) || null,
          exit_code: result.exit_code,
          signal: result.signal,
        },
        null,
        2,
      )}\n`,
    );
  }
  return result.exit_code;
}

if (require.main === module) {
  process.exit(main());
}

module.exports = {
  main,
  parseArgs,
  listEntries,
};
