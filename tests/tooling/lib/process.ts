#!/usr/bin/env node
'use strict';

import { spawnSync } from 'node:child_process';

export type ProcessResult = {
  ok: boolean;
  status: number;
  signal: string | null;
  stdout: string;
  stderr: string;
  duration_ms: number;
  timed_out: boolean;
  deferred_host_stall: boolean;
};

export function expandEnvValue(value: string, env: NodeJS.ProcessEnv): string {
  return String(value || '').replace(/\$\{([A-Z0-9_]+)\}/g, (_match, name) =>
    String(env[name] || ''),
  );
}

export function runCommand(
  command: string[],
  options: {
    cwd?: string;
    env?: NodeJS.ProcessEnv;
    timeoutSec?: number;
    deferHostStall?: boolean;
    shell?: boolean;
    inheritStdio?: boolean;
  } = {},
): ProcessResult {
  const started = Date.now();
  const timeoutMs = Math.max(1000, Math.floor((options.timeoutSec || 60) * 1000));
  const env = options.env || process.env;
  const stdio = options.inheritStdio ? 'inherit' : 'pipe';
  const child = spawnSync(command[0], command.slice(1), {
    cwd: options.cwd || process.cwd(),
    env,
    shell: options.shell || false,
    encoding: 'utf8',
    stdio,
    timeout: timeoutMs,
    maxBuffer: 10 * 1024 * 1024,
  });
  const timedOut = Boolean(child.error && (child.error as NodeJS.ErrnoException).code === 'ETIMEDOUT');
  const deferred = timedOut && Boolean(options.deferHostStall);
  const status = deferred ? 0 : child.status ?? (timedOut ? 124 : 1);
  return {
    ok: status === 0,
    status,
    signal: child.signal ?? null,
    stdout: String(child.stdout || ''),
    stderr: String(child.stderr || (child.error ? (child.error as Error).message : '')),
    duration_ms: Date.now() - started,
    timed_out: timedOut,
    deferred_host_stall: deferred,
  };
}

