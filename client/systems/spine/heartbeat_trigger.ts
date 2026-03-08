#!/usr/bin/env node
'use strict';
export {};

/**
 * systems/spine/heartbeat_trigger.js
 *
 * Compatibility shell only.
 * - Delegates run/status to spine_safe_launcher.
 * - Applies bounded memory + timeout guard for CLI stability.
 * - Preserves optional min-hours throttling for manual invocations.
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

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
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

function resolveScriptInvocation(scriptAbsPath: string) {
  if (fs.existsSync(scriptAbsPath)) return [scriptAbsPath];
  if (scriptAbsPath.endsWith('.js')) {
    const tsPath = scriptAbsPath.slice(0, -3) + '.ts';
    if (fs.existsSync(tsPath)) return [path.join(ROOT, 'lib', 'ts_entrypoint.js'), tsPath];
  }
  return [scriptAbsPath];
}

function parseJsonLinePayload(stdout: string) {
  const raw = String(stdout || '').trim();
  if (!raw) return null;
  try {
    return JSON.parse(raw);
  } catch {}
  const lines = raw.split('\n').map((line) => line.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function readJsonl(filePath: string) {
  if (!fs.existsSync(filePath)) return [];
  return fs.readFileSync(filePath, 'utf8')
    .split('\n')
    .filter(Boolean)
    .map((line) => {
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

function commandPlan() {
  const cmd = String(process.argv[2] || '').trim().toLowerCase();
  if (!cmd || cmd === '--help' || cmd === '-h' || cmd === 'help') return { command: 'help' };
  if (cmd === 'status') {
    return {
      command: 'status',
      mode: String(parseArg('mode', 'daily') || 'daily') === 'eyes' ? 'eyes' : 'daily',
      date: cleanText(parseArg('date', todayStr()), 20) || todayStr()
    };
  }
  if (cmd === 'run') {
    return {
      command: 'run',
      mode: String(parseArg('mode', 'daily') || 'daily') === 'eyes' ? 'eyes' : 'daily',
      date: todayStr(),
      minHours: toNumber(parseArg('min-hours', process.env.SPINE_HEARTBEAT_MIN_HOURS || '4'), 4, 0, 168),
      maxEyes: cleanText(parseArg('max-eyes', ''), 16)
    };
  }
  return { command: 'invalid', raw: cmd };
}

function runDelegated(args: string[], timeoutMs: number, maxOldSpaceMb: number, envExtra: Record<string, string>) {
  const invocation = resolveScriptInvocation(SAFE_LAUNCHER);
  const child = spawnSync(
    process.execPath,
    [`--max-old-space-size=${Math.floor(maxOldSpaceMb)}`, ...invocation, ...args],
    {
      cwd: ROOT,
      encoding: 'utf8',
      timeout: timeoutMs,
      killSignal: 'SIGTERM',
      env: {
        ...process.env,
        ...envExtra
      }
    }
  );
  return {
    status: Number.isFinite(child.status) ? Number(child.status) : 1,
    signal: child.signal ? String(child.signal) : null,
    timedOut: !!(child.error && String(child.error.code || '') === 'ETIMEDOUT'),
    stdout: String(child.stdout || ''),
    stderr: String(child.stderr || ''),
    payload: parseJsonLinePayload(String(child.stdout || ''))
  };
}

async function main() {
  const plan = commandPlan();
  if (plan.command === 'help') {
    usage();
    process.exit(0);
  }
  if (plan.command === 'invalid') {
    usage();
    process.exit(2);
  }

  const timeoutMs = toNumber(process.env.SPINE_HEARTBEAT_TRIGGER_TIMEOUT_MS, DEFAULT_TIMEOUT_MS, 5000, 10 * 60 * 1000);
  const maxOldSpaceMb = toNumber(process.env.SPINE_HEARTBEAT_TRIGGER_MAX_OLD_SPACE_MB, DEFAULT_MAX_OLD_SPACE_MB, 96, 1024);

  if (plan.command === 'run') {
    const last = lastSpineRunStarted(plan.mode, plan.date);
    const nowMs = Date.now();
    const lastMs = last ? Date.parse(String(last.ts || '')) : NaN;
    const hoursSince = Number.isFinite(lastMs) ? ((nowMs - lastMs) / (1000 * 60 * 60)) : null;
    if (last && Number.isFinite(hoursSince) && hoursSince! < plan.minHours) {
      process.stdout.write(`${JSON.stringify({
        ok: true,
        type: 'heartbeat_trigger_compat',
        compatibility_shell: true,
        authority: 'rust_spine',
        delegated_to: 'spine_safe_launcher',
        result: 'skipped_recent_run',
        mode: plan.mode,
        date: plan.date,
        min_hours: plan.minHours,
        hours_since_last: Number(hoursSince!.toFixed(3)),
        last_run_ts: String(last.ts || ''),
        ts: nowIso()
      })}\n`);
      process.exit(0);
    }
    const delegatedArgs = ['run', plan.mode, plan.date];
    if (plan.maxEyes) delegatedArgs.push(`--max-eyes=${plan.maxEyes}`);
    const result = runDelegated(delegatedArgs, timeoutMs, maxOldSpaceMb, {
      SPINE_RUN_CONTEXT: process.env.SPINE_RUN_CONTEXT || 'heartbeat_cli',
      SPINE_HEARTBEAT_COMPAT_SHELL: '1'
    });
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    if (result.status !== 0) {
      process.stderr.write(
        `heartbeat_trigger_delegation_failed:status=${result.status} timed_out=${result.timedOut ? '1' : '0'} signal=${cleanText(result.signal || '', 40)}\n`
      );
    }
    process.exit(result.status);
  }

  const statusResult = runDelegated(
    ['status', `--mode=${plan.mode}`, `--date=${plan.date}`],
    timeoutMs,
    maxOldSpaceMb,
    {
      SPINE_RUN_CONTEXT: process.env.SPINE_RUN_CONTEXT || 'heartbeat_cli',
      SPINE_HEARTBEAT_COMPAT_SHELL: '1'
    }
  );
  if (statusResult.stdout) process.stdout.write(statusResult.stdout);
  if (statusResult.stderr) process.stderr.write(statusResult.stderr);
  process.exit(statusResult.status);
}

main().catch((err: any) => {
  process.stderr.write(`heartbeat_trigger_unhandled:${cleanText(err && err.message ? err.message : err, 280)}\n`);
  process.exit(1);
});
