#!/usr/bin/env node
'use strict';
export {};

const { execSync } = require('child_process');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '').trim();
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const eq = token.indexOf('=');
    if (eq >= 0) {
      out[token.slice(2, eq)] = token.slice(eq + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = '1';
  }
  return out;
}

function toBool(v: unknown, fallback = false) {
  const raw = cleanText(v, 20).toLowerCase();
  if (!raw) return fallback;
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function toInt(v: unknown, fallback: number, lo: number, hi: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(lo, Math.min(hi, Math.floor(n)));
}

function etimeToSeconds(etime: string) {
  const raw = cleanText(etime, 40);
  if (!raw) return 0;
  let days = 0;
  let rest = raw;
  if (raw.includes('-')) {
    const [d, r] = raw.split('-', 2);
    days = Number(d) || 0;
    rest = r || '0:00';
  }
  const parts = rest.split(':').map((x) => Number(x) || 0);
  let h = 0; let m = 0; let s = 0;
  if (parts.length === 3) {
    [h, m, s] = parts;
  } else if (parts.length === 2) {
    [m, s] = parts;
  } else if (parts.length === 1) {
    [s] = parts;
  }
  return (days * 86400) + (h * 3600) + (m * 60) + s;
}

function listProcesses() {
  const out = execSync('ps -axo pid,ppid,etime,command', { encoding: 'utf8' });
  return String(out || '')
    .split('\n')
    .slice(1)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const match = line.match(/^(\d+)\s+(\d+)\s+(\S+)\s+(.+)$/);
      if (!match) return null;
      return {
        pid: Number(match[1]),
        ppid: Number(match[2]),
        etime: match[3],
        age_sec: etimeToSeconds(match[3]),
        command: match[4]
      };
    })
    .filter(Boolean) as Array<{pid:number, ppid:number, etime:string, age_sec:number, command:string}>;
}

function buildStaleSet(rows: Array<{pid:number, ppid:number, age_sec:number, command:string}>, maxAgeSec: number) {
  const staleBuildScripts = rows.filter((row) => row.command.includes('build-script-build') && row.age_sec >= maxAgeSec);
  const staleCargo = new Set<number>();
  for (const row of staleBuildScripts) staleCargo.add(row.ppid);
  const staleCargoRows = rows.filter((row) => staleCargo.has(row.pid) && row.command.includes('/cargo'));
  return { staleBuildScripts, staleCargoRows };
}

function killPid(pid: number) {
  try {
    process.kill(pid, 'SIGTERM');
    return true;
  } catch {
    return false;
  }
}

function run() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'check', 20).toLowerCase();
  const maxAgeSec = toInt(args['max-age-sec'] || process.env.HOST_BUILD_STALE_MAX_AGE_SEC, 90, 10, 3600);
  const applyKill = cmd === 'reap' || toBool(args.apply, false) || toBool(args.kill, false);
  const rows = listProcesses();
  const stale = buildStaleSet(rows, maxAgeSec);
  const stalePids = [
    ...stale.staleBuildScripts.map((row) => row.pid),
    ...stale.staleCargoRows.map((row) => row.pid)
  ];
  const uniqueStalePids = Array.from(new Set(stalePids));
  const killed: number[] = [];
  if (applyKill) {
    for (const pid of uniqueStalePids) {
      if (killPid(pid)) killed.push(pid);
    }
  }
  const payload = {
    ok: true,
    type: 'host_build_stale_guard',
    ts: nowIso(),
    command: cmd,
    max_age_sec: maxAgeSec,
    stale_detected: uniqueStalePids.length > 0,
    stale_count: uniqueStalePids.length,
    stale_build_scripts: stale.staleBuildScripts.map((row) => ({
      pid: row.pid,
      ppid: row.ppid,
      age_sec: row.age_sec,
      command: cleanText(row.command, 180)
    })),
    stale_cargo_parents: stale.staleCargoRows.map((row) => ({
      pid: row.pid,
      ppid: row.ppid,
      age_sec: row.age_sec,
      command: cleanText(row.command, 180)
    })),
    killed_pids: killed,
    reason_code: uniqueStalePids.length > 0 ? 'stale_build_script_detected' : 'none'
  };
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  // check exits 2 on stale so wrappers can fail-fast.
  if (cmd === 'check' && uniqueStalePids.length > 0) process.exit(2);
  process.exit(0);
}

run();

