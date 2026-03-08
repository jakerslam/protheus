#!/usr/bin/env node
'use strict';
export {};

const { spawn, execSync } = require('child_process');
const fs = require('fs');
const path = require('path');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 300) {
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
  if (parts.length === 3) [h, m, s] = parts;
  else if (parts.length === 2) [m, s] = parts;
  else if (parts.length === 1) [s] = parts;
  return (days * 86400) + (h * 3600) + (m * 60) + s;
}

function listBuildScriptsForParent(parentPid: number) {
  const out = execSync('ps -axo pid,ppid,etime,rss,command', { encoding: 'utf8' });
  return String(out || '')
    .split('\n')
    .slice(1)
    .map((line) => line.trim())
    .filter(Boolean)
    .map((line) => {
      const match = line.match(/^(\d+)\s+(\d+)\s+(\S+)\s+(\d+)\s+(.+)$/);
      if (!match) return null;
      return {
        pid: Number(match[1]),
        ppid: Number(match[2]),
        etime: match[3],
        age_sec: etimeToSeconds(match[3]),
        rss_kb: Number(match[4]) || 0,
        command: match[5]
      };
    })
    .filter((row) => !!row)
    .filter((row: any) => row.ppid === parentPid && String(row.command).includes('build-script-build')) as Array<{pid:number,ppid:number,etime:string,age_sec:number,rss_kb:number,command:string}>;
}

function writeLatest(payload: Record<string, any>) {
  try {
    const outPath = path.join(process.cwd(), 'client', 'runtime', 'local', 'state', 'ops', 'host_rust_validation', 'latest.json');
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  } catch {
    // best-effort artifact only
  }
}

function shouldDeferHostStall(
  payload: Record<string, any>,
  deferOnHostStall: boolean
) {
  if (!deferOnHostStall) return false;
  const reason = cleanText(payload.reason_code || '', 120);
  return reason === 'dyld_loader_stall_detected' || reason === 'stale_build_script_detected';
}

function profileToCargoArgs(profile: string) {
  const key = cleanText(profile, 64).toLowerCase();
  if (key === 'protheus_ops_attention') {
    return ['test', '-p', 'protheus-ops-core', 'attention_queue', '--', '--nocapture'];
  }
  if (key === 'execution_core_initiative') {
    return ['test', '-p', 'execution_core', 'initiative', '--', '--nocapture'];
  }
  throw new Error(`unsupported_profile:${key}`);
}

function rowHasNotExceededGrace(
  row: { age_sec: number },
  loaderStallAgeSec: number,
  loaderStallLockGraceSec: number
) {
  const hardLimit = Math.max(loaderStallAgeSec, 0) + Math.max(loaderStallLockGraceSec, 0);
  return Number(row && row.age_sec || 0) <= hardLimit;
}

async function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function runValidationAttempt(
  profile: string,
  cargoArgs: string[],
  staleAgeSec: number,
  loaderStallAgeSec: number,
  loaderStallRssKbMax: number,
  loaderStallLockGraceSec: number,
  idleThresholdMs: number,
  checkIntervalMs: number,
  timeoutMs: number
) {
  const startedAt = Date.now();
  const targetDir = path.join(process.cwd(), 'client', 'runtime', 'local', 'state', 'ops', 'host_rust_validation', 'target', profile);
  const child = spawn('cargo', cargoArgs, {
    cwd: process.cwd(),
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir
    },
    stdio: ['ignore', 'pipe', 'pipe']
  });
  let stdout = '';
  let stderr = '';
  let lastProgressMs = Date.now();
  child.stdout.on('data', (chunk: Buffer) => {
    stdout += String(chunk || '');
    lastProgressMs = Date.now();
    process.stdout.write(chunk);
  });
  child.stderr.on('data', (chunk: Buffer) => {
    stderr += String(chunk || '');
    lastProgressMs = Date.now();
    process.stderr.write(chunk);
  });

  let staleDetected = null as any;
  let loaderStallDetected = null as any;
  let timeoutTriggered = false;
  while (true) {
    const finished = child.exitCode != null;
    if (finished) break;
    if ((Date.now() - startedAt) > timeoutMs) {
      timeoutTriggered = true;
      try { process.kill(child.pid, 'SIGTERM'); } catch {}
      break;
    }
    const staleRows = listBuildScriptsForParent(child.pid);
    const idleMs = Date.now() - lastProgressMs;
    if (idleMs < idleThresholdMs) {
      await sleep(checkIntervalMs);
      continue;
    }
    const stderrTail = String(stderr || '').slice(-4000);
    const cargoLockWaitDetected =
      /Blocking waiting for file lock on package cache/i.test(stderrTail)
      || /Blocking waiting for file lock on build directory/i.test(stderrTail);
    const loaderStall = staleRows.find((row) => row.age_sec >= loaderStallAgeSec && row.rss_kb > 0 && row.rss_kb <= loaderStallRssKbMax);
    if (loaderStall) {
      const protectedByLockWait =
        cargoLockWaitDetected && rowHasNotExceededGrace(loaderStall, loaderStallAgeSec, loaderStallLockGraceSec);
      if (protectedByLockWait) {
        await sleep(checkIntervalMs);
        continue;
      }
      loaderStallDetected = loaderStall;
      try { process.kill(loaderStall.pid, 'SIGTERM'); } catch {}
      try { process.kill(child.pid, 'SIGTERM'); } catch {}
      break;
    }
    const stale = staleRows.find((row) => row.age_sec >= staleAgeSec);
    if (stale) {
      staleDetected = stale;
      try { process.kill(stale.pid, 'SIGTERM'); } catch {}
      try { process.kill(child.pid, 'SIGTERM'); } catch {}
      break;
    }
    await sleep(checkIntervalMs);
  }

  await sleep(250);
  const exitCode = Number.isFinite(child.exitCode)
    ? Number(child.exitCode)
    : (timeoutTriggered || staleDetected || loaderStallDetected ? 124 : 1);
  const payload = {
    ok: exitCode === 0,
    type: 'host_rust_validation',
    ts: nowIso(),
    profile,
    command: ['cargo', ...cargoArgs],
    elapsed_ms: Date.now() - startedAt,
    stale_age_sec: staleAgeSec,
    loader_stall_age_sec: loaderStallAgeSec,
    loader_stall_rss_kb_max: loaderStallRssKbMax,
    idle_threshold_ms: idleThresholdMs,
    timeout_ms: timeoutMs,
    stale_detected: !!staleDetected,
    loader_stall_detected: !!loaderStallDetected,
    stale_process: staleDetected
      ? { pid: staleDetected.pid, age_sec: staleDetected.age_sec, command: cleanText(staleDetected.command, 180) }
      : null,
    loader_stall_process: loaderStallDetected
      ? { pid: loaderStallDetected.pid, age_sec: loaderStallDetected.age_sec, rss_kb: loaderStallDetected.rss_kb, command: cleanText(loaderStallDetected.command, 180) }
      : null,
    timeout_triggered: timeoutTriggered,
    exit_code: exitCode,
    reason_code: loaderStallDetected
      ? 'dyld_loader_stall_detected'
      : (staleDetected
        ? 'stale_build_script_detected'
        : (timeoutTriggered ? 'validation_timeout' : (exitCode === 0 ? 'none' : 'validation_failed'))),
    stderr_tail: cleanText(stderr.slice(-1000), 1000),
    stdout_tail: cleanText(stdout.slice(-1000), 1000)
  };
  return payload;
}

function reapStaleBuildScripts(maxAgeSec: number) {
  try {
    const cmd = `node client/runtime/systems/ops/host_build_stale_guard.js reap --kill=1 --max-age-sec=${Math.max(10, maxAgeSec)}`;
    execSync(cmd, { cwd: process.cwd(), stdio: ['ignore', 'ignore', 'ignore'] });
  } catch {
    // best-effort cleanup only
  }
}

async function run() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'run', 20).toLowerCase();
  if (cmd !== 'run') {
    process.stdout.write(`${JSON.stringify({ ok: false, error: 'unsupported_command', command: cmd })}\n`);
    process.exit(2);
  }
  const profile = cleanText(args.profile || '', 80) || 'protheus_ops_attention';
  const cargoArgs = profileToCargoArgs(profile);
  const staleAgeSec = toInt(args['stale-age-sec'] || process.env.HOST_BUILD_STALE_MAX_AGE_SEC, 90, 20, 3600);
  const loaderStallAgeSec = toInt(args['loader-stall-age-sec'] || process.env.HOST_BUILD_LOADER_STALL_AGE_SEC, 25, 5, 300);
  const loaderStallRssKbMax = toInt(args['loader-stall-rss-kb-max'] || process.env.HOST_BUILD_LOADER_STALL_RSS_KB_MAX, 256, 64, 2048);
  const loaderStallLockGraceSec = toInt(
    args['loader-stall-lock-grace-sec'] || process.env.HOST_BUILD_LOADER_STALL_LOCK_GRACE_SEC,
    180,
    0,
    1800
  );
  const idleThresholdMs = toInt(
    args['idle-threshold-ms'] || process.env.HOST_BUILD_IDLE_THRESHOLD_MS,
    120000,
    5000,
    1800000
  );
  const checkIntervalMs = toInt(args['check-interval-ms'] || 5000, 1000, 1000, 60000);
  const timeoutMs = toInt(args['timeout-ms'] || 20 * 60 * 1000, 10000, 10000, 2 * 60 * 60 * 1000);
  const maxRetries = toInt(args['max-retries'] || process.env.HOST_RUST_VALIDATION_MAX_RETRIES, 1, 0, 5);
  const preflightReap = (String(args['preflight-reap'] || process.env.HOST_RUST_VALIDATION_PREFLIGHT_REAP || '1').trim() !== '0');
  const deferOnHostStall = (String(args['defer-on-host-stall'] || process.env.HOST_RUST_VALIDATION_DEFER_ON_HOST_STALL || '0').trim() !== '0');
  if (preflightReap) {
    reapStaleBuildScripts(staleAgeSec);
    await sleep(750);
  }

  const attempts: Array<{ attempt: number, reason_code: string, exit_code: number }> = [];
  let payload = null as any;
  for (let attempt = 1; attempt <= (maxRetries + 1); attempt += 1) {
    payload = await runValidationAttempt(
      profile,
      cargoArgs,
      staleAgeSec,
      loaderStallAgeSec,
      loaderStallRssKbMax,
      loaderStallLockGraceSec,
      idleThresholdMs,
      checkIntervalMs,
      timeoutMs
    );
    attempts.push({
      attempt,
      reason_code: cleanText(payload.reason_code, 120),
      exit_code: Number(payload.exit_code ?? 1)
    });
    if (payload.exit_code === 0) break;
    const canRetry = (
      payload.reason_code === 'stale_build_script_detected'
      || payload.reason_code === 'dyld_loader_stall_detected'
    ) && attempt <= maxRetries;
    if (!canRetry) break;
    reapStaleBuildScripts(staleAgeSec);
    await sleep(1200);
  }

  payload.attempts = attempts;
  payload.retried = attempts.length > 1;
  payload.max_retries = maxRetries;
  payload.preflight_reap = preflightReap;
  payload.defer_on_host_stall = deferOnHostStall;
  if (shouldDeferHostStall(payload, deferOnHostStall)) {
    payload.deferred = true;
    payload.deferred_reason = 'host_stall';
    payload.raw_reason_code = cleanText(payload.reason_code, 120) || 'unknown';
    payload.reason_code = 'deferred_host_stall';
    payload.ok = true;
    payload.exit_code = 0;
  } else {
    payload.deferred = false;
  }
  writeLatest(payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  process.exit(Number(payload.exit_code ?? 1));
}

run().catch((err: any) => {
  process.stderr.write(`host_rust_validation_error:${cleanText(err && err.message ? err.message : err, 300)}\n`);
  process.exit(1);
});
