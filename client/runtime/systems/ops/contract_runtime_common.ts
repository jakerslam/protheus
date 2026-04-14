#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

export type CommonArgs = {
  command: string;
  strict: boolean;
  json: boolean;
  policy: string;
};

export function clean(value: unknown, max = 400): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

export function parseCommonArgs(argv: string[], defaults: { command?: string; policy: string }): CommonArgs {
  const out: CommonArgs = {
    command: clean(defaults.command || argv[0] || 'check', 32).toLowerCase(),
    strict: false,
    json: false,
    policy: defaults.policy,
  };
  for (const token of argv.slice(1)) {
    const value = clean(token, 600);
    if (!value) continue;
    if (value === '--strict' || value === '--strict=1' || value === '--strict=true') out.strict = true;
    else if (value === '--json' || value === '--json=1' || value === '--json=true') out.json = true;
    else if (value.startsWith('--policy=')) out.policy = clean(value.slice('--policy='.length), 260);
  }
  return out;
}

export function readJson<T>(filePath: string): T {
  return JSON.parse(fs.readFileSync(path.resolve(filePath), 'utf8')) as T;
}

export function pathExists(filePath: string): boolean {
  return fs.existsSync(path.resolve(filePath));
}

export function writeJsonArtifact(filePath: string, payload: unknown): void {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

export function appendJsonLine(filePath: string, payload: unknown): void {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.appendFileSync(abs, `${JSON.stringify(payload)}\n`, 'utf8');
}

export function currentRevision(cwd = process.cwd()): string {
  try {
    return clean(execSync('git rev-parse HEAD', { cwd, encoding: 'utf8' }), 120) || 'unknown';
  } catch {
    return 'unknown';
  }
}

export function normalizePath(value: string): string {
  return clean(value, 600).replace(/\\/g, '/');
}

export function listTrackedFiles(cwd = process.cwd()): string[] {
  try {
    const raw = execSync('git ls-files', { cwd, encoding: 'utf8' });
    return String(raw || '')
      .split('\n')
      .map((line) => normalizePath(line))
      .filter(Boolean);
  } catch {
    return [];
  }
}

export function emitContractResult(
  payload: Record<string, unknown>,
  options: {
    strict: boolean;
    latestPath: string;
    receiptsPath: string;
  },
): number {
  writeJsonArtifact(options.latestPath, payload);
  appendJsonLine(options.receiptsPath, payload);
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  return options.strict && payload.ok === false ? 1 : 0;
}
