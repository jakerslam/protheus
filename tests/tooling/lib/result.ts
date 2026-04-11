#!/usr/bin/env node
'use strict';

import fs from 'node:fs';
import path from 'node:path';
import { toolingHistoryPath } from './artifacts.ts';

export function writeJsonArtifact(filePath: string, payload: unknown): void {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

export function writeTextArtifact(filePath: string, payload: string): void {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, payload, 'utf8');
}

export function appendJsonLine(filePath: string, payload: unknown): void {
  const abs = path.resolve(filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.appendFileSync(abs, `${JSON.stringify(payload)}\n`, 'utf8');
}

export function emitStructuredResult(
  payload: unknown,
  options: {
    outPath?: string;
    strict?: boolean;
    ok?: boolean;
    history?: boolean;
    stdout?: boolean;
  } = {},
): number {
  if (options.outPath) writeJsonArtifact(options.outPath, payload);
  if (options.history !== false) appendJsonLine(toolingHistoryPath(), payload);
  if (options.stdout !== false) {
    process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  }
  if (options.strict && options.ok === false) return 1;
  return 0;
}
