#!/usr/bin/env node
'use strict';

import { runCommand } from './process.ts';

export function currentRevision(cwd = process.cwd()): string {
  const result = runCommand(['git', 'rev-parse', 'HEAD'], {
    cwd,
    timeoutSec: 15,
  });
  return result.ok ? String(result.stdout || '').trim() || 'unknown' : 'unknown';
}

export function trackedFiles(cwd = process.cwd()): string[] {
  const result = runCommand(['git', 'ls-files'], {
    cwd,
    timeoutSec: 30,
  });
  if (!result.ok) return [];
  return String(result.stdout || '')
    .split('\n')
    .map((line) => line.trim().replace(/\\/g, '/'))
    .filter(Boolean);
}
