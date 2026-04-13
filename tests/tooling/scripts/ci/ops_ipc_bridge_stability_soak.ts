#!/usr/bin/env node
import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';

type SoakRow = {
  iteration: number;
  ok: boolean;
  status: number;
  duration_ms: number;
  routed_via: string;
  payload_type: string;
  daemon_pid: number;
  notes: string[];
};

type ParsedArgs = {
  iterations: number;
  killAt: number;
  timeoutMs: number;
  pollMs: number;
};

const ROOT = process.cwd();
const ARTIFACT_DIR = path.join(ROOT, 'artifacts');
const STATE_DIR = path.join(ROOT, 'local', 'state', 'ops', 'ops_ipc_bridge_stability_soak');
const LATEST_STATE_PATH = path.join(STATE_DIR, 'latest.json');
const BRIDGE = require('../../../../client/runtime/lib/rust_lane_bridge.ts');

function parseIntFlag(name: string, fallback: number, min: number, max: number): number {
  const match = process.argv
    .slice(2)
    .map((row) => String(row || '').trim())
    .find((row) => row.startsWith(`--${name}=`));
  const raw = match ? Number.parseInt(match.split('=').slice(1).join('='), 10) : Number.NaN;
  if (!Number.isFinite(raw)) return fallback;
  return Math.max(min, Math.min(max, raw));
}

function parseArgs(): ParsedArgs {
  const iterations = parseIntFlag('iterations', 18, 3, 250);
  const timeoutMs = parseIntFlag('timeout-ms', 45_000, 5_000, 300_000);
  const pollMs = parseIntFlag('poll-ms', 20, 5, 1_000);
  const killAt = parseIntFlag('kill-at', Math.max(2, Math.floor(iterations / 2)), 2, iterations - 1);
  return { iterations, killAt, timeoutMs, pollMs };
}

function nowIso(): string {
  return new Date().toISOString();
}

function slug(iso: string): string {
  return iso.replaceAll(':', '-').replaceAll('.', '-');
}

function writeJson(absPath: string, payload: unknown): void {
  fs.mkdirSync(path.dirname(absPath), { recursive: true });
  fs.writeFileSync(absPath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
}

function queueDirForRoot(root: string): string {
  const hash = crypto.createHash('sha256').update(root).digest('hex').slice(0, 16);
  return path.join(root, 'local', 'state', 'tools', 'ops_bridge_ipc', hash);
}

function daemonPidFile(queueDir: string): string {
  return path.join(queueDir, 'daemon.pid.json');
}

function readDaemonPid(queueDir: string): number {
  try {
    const parsed = JSON.parse(fs.readFileSync(daemonPidFile(queueDir), 'utf8'));
    const pid = Number(parsed?.pid || 0);
    return Number.isFinite(pid) ? pid : 0;
  } catch {
    return 0;
  }
}

function killDaemon(queueDir: string): { killed: boolean; pid: number; reason: string } {
  const pid = readDaemonPid(queueDir);
  if (!pid || pid <= 0) {
    return { killed: false, pid: 0, reason: 'pid_missing' };
  }
  try {
    process.kill(pid, 'SIGTERM');
    return { killed: true, pid, reason: 'killed' };
  } catch (err) {
    return {
      killed: false,
      pid,
      reason: String(err instanceof Error ? err.message : err),
    };
  }
}

function listJsonFiles(absDir: string): string[] {
  try {
    return fs
      .readdirSync(absDir)
      .filter((name) => name.endsWith('.json'))
      .map((name) => path.join(absDir, name))
      .sort();
  } catch {
    return [];
  }
}

function removeFiles(paths: string[]): string[] {
  const removed: string[] = [];
  for (const abs of paths) {
    try {
      fs.rmSync(abs, { force: true });
      removed.push(abs);
    } catch {
      // keep moving; residual check below catches failures
    }
  }
  return removed;
}

function main(): number {
  const startedAt = nowIso();
  const parsed = parseArgs();
  process.env.INFRING_OPS_IPC_DAEMON = process.env.INFRING_OPS_IPC_DAEMON || '1';
  process.env.INFRING_OPS_IPC_STRICT = process.env.INFRING_OPS_IPC_STRICT || '1';
  process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = String(parsed.timeoutMs);
  process.env.INFRING_OPS_IPC_POLL_MS = String(parsed.pollMs);
  process.env.INFRING_OPS_IPC_STALE_MS = process.env.INFRING_OPS_IPC_STALE_MS || '150';

  const bridge = BRIDGE.createOpsLaneBridge(
    __dirname,
    'ops_ipc_bridge_stability_soak',
    'command-list-kernel',
  );

  const queueDir = queueDirForRoot(ROOT);
  const requestsDir = path.join(queueDir, 'requests');
  const responsesDir = path.join(queueDir, 'responses');
  fs.mkdirSync(requestsDir, { recursive: true });
  fs.mkdirSync(responsesDir, { recursive: true });
  const baselineRequests = new Set(listJsonFiles(requestsDir));
  const baselineResponses = new Set(listJsonFiles(responsesDir));

  const rows: SoakRow[] = [];
  const restartSignals: string[] = [];

  for (let iteration = 1; iteration <= parsed.iterations; iteration += 1) {
    const notes: string[] = [];
    if (iteration === parsed.killAt) {
      const kill = killDaemon(queueDir);
      notes.push(`daemon_kill:${kill.reason}`);
      if (kill.killed) {
        restartSignals.push(`killed_pid:${kill.pid}`);
      }
    }

    const started = Date.now();
    const out = bridge.run(['--mode=list', '--json']);
    const durationMs = Date.now() - started;
    const daemonPid = readDaemonPid(queueDir);
    const payloadType =
      out && out.payload && typeof out.payload === 'object' ? String(out.payload.type || '') : '';
    const routedVia = String(
      out?.routed_via ||
      (out && out.payload && typeof out.payload === 'object' ? String(out.payload.routed_via || '') : '') ||
      '',
    );
    const status = Number.isFinite(Number(out?.status)) ? Number(out.status) : 1;
    const ok =
      status === 0 &&
      out?.ok === true &&
      (routedVia === 'ipc_daemon' || routedVia === 'conduit');
    if (!ok) {
      notes.push(`status=${status}`);
      notes.push(`routed_via=${routedVia || 'unknown'}`);
      notes.push(`payload_type=${payloadType || 'unknown'}`);
    }
    rows.push({
      iteration,
      ok,
      status,
      duration_ms: durationMs,
      routed_via: routedVia,
      payload_type: payloadType,
      daemon_pid: daemonPid,
      notes,
    });
  }

  const requestBacklog = listJsonFiles(requestsDir).filter((row) => !baselineRequests.has(row));
  const responseBacklog = listJsonFiles(responsesDir).filter((row) => !baselineResponses.has(row));
  const reapedResponses = removeFiles(responseBacklog);
  const responseBacklogAfterReap = listJsonFiles(responsesDir).filter((row) => !baselineResponses.has(row));
  const pidSet = new Set(rows.map((row) => row.daemon_pid).filter((pid) => pid > 0));
  if (pidSet.size >= 2) {
    restartSignals.push('pid_rotation_detected');
  }
  const failedRows = rows.filter((row) => !row.ok);
  const ok =
    failedRows.length === 0 &&
    requestBacklog.length === 0 &&
    responseBacklogAfterReap.length === 0;

  const report = {
    type: 'ops_ipc_bridge_stability_soak',
    schema_version: 1,
    started_at: startedAt,
    finished_at: nowIso(),
    config: parsed,
    ok,
    checks: {
      all_iterations_ok: failedRows.length === 0,
      request_backlog_empty: requestBacklog.length === 0,
      response_backlog_empty_before_reap: responseBacklog.length === 0,
      response_backlog_empty_after_reap: responseBacklogAfterReap.length === 0,
      daemon_restart_signal_detected: restartSignals.length > 0,
    },
    daemon_restart_signals: restartSignals,
    daemon_pids_seen: Array.from(pidSet),
    failures: failedRows,
    backlog: {
      requests: requestBacklog,
      responses_before_reap: responseBacklog,
      responses_reaped: reapedResponses,
      responses_after_reap: responseBacklogAfterReap,
    },
    rows,
    command: 'node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/ops_ipc_bridge_stability_soak.ts',
  };

  fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
  const stampedPath = path.join(
    ARTIFACT_DIR,
    `ops_ipc_bridge_stability_soak_report_${slug(report.finished_at)}.json`,
  );
  const latestPath = path.join(ARTIFACT_DIR, 'ops_ipc_bridge_stability_soak_report_latest.json');
  writeJson(stampedPath, report);
  writeJson(latestPath, report);
  writeJson(LATEST_STATE_PATH, report);

  process.stdout.write(`${JSON.stringify(report)}\n`);
  return ok ? 0 : 1;
}

process.exit(main());
