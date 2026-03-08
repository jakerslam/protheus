#!/usr/bin/env node
'use strict';
export {};

/**
 * systems/spine/heartbeat_trigger.js
 *
 * Compatibility shell only.
 * - Delegates run/status to spine_safe_launcher.
 * - Uses bounded timeout/memory for CLI stability.
 * - Keeps optional min-hours throttling for manual invocations.
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const SAFE_LAUNCHER = path.join(ROOT, 'systems', 'spine', 'spine_safe_launcher.js');
const DEFAULT_TIMEOUT_MS = Math.max(
  5000,
  Math.min(10 * 60 * 1000, Number(process.env.SPINE_HEARTBEAT_TRIGGER_TIMEOUT_MS || 30000) || 30000)
);
const DEFAULT_MAX_OLD_SPACE_MB = Math.max(
  96,
  Math.min(1024, Number(process.env.SPINE_HEARTBEAT_TRIGGER_MAX_OLD_SPACE_MB || 192) || 192)
);

function nowIso() {
  return new Date().toISOString();
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function toNumber(v: unknown, fallback: number, lo: number, hi: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(lo, Math.min(hi, n));
}

function parseArg(name: string, fallback: string | null = null) {
  const pref = `--${name}=`;
  const arg = process.argv.find((token) => String(token).startsWith(pref));
  return arg ? String(arg).slice(pref.length) : fallback;
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/spine/heartbeat_trigger.js run [--mode=eyes|daily] [--min-hours=N] [--max-eyes=N]');
  console.log('  node systems/spine/heartbeat_trigger.js status [--mode=eyes|daily] [--date=YYYY-MM-DD]');
  console.log('  node systems/spine/heartbeat_trigger.js --help');
}

function readJsonl(filePath: string) {
  if (!fs.existsSync(filePath)) return [];
  return fs
    .readFileSync(filePath, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((line: string) => {
      try {
        return JSON.parse(line);
      } catch {
        return null;
      }
    })
    .filter(Boolean);
}

function lastSpineRunStarted(mode: string, dateStr: string) {
  const fp = path.join(ROOT, 'local', 'state', 'spine', 'runs', `${dateStr}.jsonl`);
  const accepted = new Set(['spine_run_started', 'spine_run_complete', 'spine_benchmark_noop']);
  const events = readJsonl(fp)
    .filter((row: any) => row && accepted.has(String(row.type || '')) && String(row.mode || '') === mode)
    .sort((a: any, b: any) => String(a.ts || '').localeCompare(String(b.ts || '')));
  return events.length > 0 ? events[events.length - 1] : null;
}

function buildDelegatedArgs(cmd: string) {
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help') return ['--help'];
  if (cmd === 'status') {
    const mode = String(parseArg('mode', 'daily') || 'daily') === 'eyes' ? 'eyes' : 'daily';
    const date = String(parseArg('date', todayStr()) || todayStr()).slice(0, 20);
    return ['status', `--mode=${mode}`, `--date=${date}`];
  }
  if (cmd === 'run') {
    const mode = String(parseArg('mode', 'daily') || 'daily') === 'eyes' ? 'eyes' : 'daily';
    const date = todayStr();
    const maxEyes = String(parseArg('max-eyes', '') || '').slice(0, 16);
    const delegated = ['run', mode, date];
    if (maxEyes) delegated.push(`--max-eyes=${maxEyes}`);
    return delegated;
  }
  return ['--help'];
}

function runDelegated(args: string[], timeoutMs: number, maxOldSpaceMb: number) {
  const child = spawnSync(
    process.execPath,
    [`--max-old-space-size=${Math.floor(maxOldSpaceMb)}`, SAFE_LAUNCHER, ...args],
    {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: timeoutMs,
      killSignal: 'SIGTERM',
      env: {
        ...process.env,
        SPINE_RUN_CONTEXT: process.env.SPINE_RUN_CONTEXT || 'heartbeat_cli',
        SPINE_HEARTBEAT_COMPAT_SHELL: '1'
      }
    }
  );
  return {
    status: Number.isFinite(child.status) ? Number(child.status) : 1,
    signal: child.signal ? String(child.signal) : null,
    timedOut: !!(child.error && String(child.error.code || '') === 'ETIMEDOUT'),
    stdout: String(child.stdout || ''),
    stderr: String(child.stderr || '')
  };
}

function main() {
  const cmd = String(process.argv[2] || '').trim().toLowerCase();
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help') {
    usage();
    process.exit(0);
  }

  const timeoutMs = DEFAULT_TIMEOUT_MS;
  const maxOldSpaceMb = DEFAULT_MAX_OLD_SPACE_MB;

  if (cmd === 'run') {
    const mode = String(parseArg('mode', 'daily') || 'daily') === 'eyes' ? 'eyes' : 'daily';
    const date = todayStr();
    const minHours = toNumber(parseArg('min-hours', process.env.SPINE_HEARTBEAT_MIN_HOURS || '4'), 4, 0, 168);
    const last = lastSpineRunStarted(mode, date);
    if (last) {
      const lastMs = Date.parse(String(last.ts || ''));
      if (Number.isFinite(lastMs)) {
        const hoursSince = (Date.now() - lastMs) / (1000 * 60 * 60);
        if (hoursSince < minHours) {
          process.stdout.write(
            `${JSON.stringify({
              ok: true,
              type: 'heartbeat_trigger_compat',
              compatibility_shell: true,
              authority: 'rust_spine',
              delegated_to: 'spine_safe_launcher',
              result: 'skipped_recent_run',
              mode,
              date,
              min_hours: minHours,
              hours_since_last: Number(hoursSince.toFixed(3)),
              last_run_ts: String(last.ts || ''),
              ts: nowIso()
            })}\n`
          );
          process.exit(0);
        }
      }
    }
  }

  const delegated = runDelegated(buildDelegatedArgs(cmd), timeoutMs, maxOldSpaceMb);
  if (delegated.stdout) process.stdout.write(delegated.stdout);
  if (delegated.stderr) process.stderr.write(delegated.stderr);
  if (delegated.timedOut) {
    process.stderr.write('heartbeat_trigger_timeout: delegated launcher exceeded timeout\n');
    process.exit(124);
  }
  if (delegated.status !== 0 && delegated.signal) {
    process.stderr.write(`heartbeat_trigger_failed: signal=${delegated.signal}\n`);
  }
  process.exit(delegated.status);
}

main();
