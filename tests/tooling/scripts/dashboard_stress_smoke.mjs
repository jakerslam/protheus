#!/usr/bin/env node
import { performance } from 'node:perf_hooks';
import process from 'node:process';
import { setTimeout as sleep } from 'node:timers/promises';
import WebSocket from 'ws';

const BASE = process.env.DASHBOARD_BASE_URL || 'http://127.0.0.1:4173';
const WS_BASE = BASE.replace(/^http/i, 'ws');

function percentile(values, p) {
  if (!values.length) return 0;
  const sorted = [...values].sort((a, b) => a - b);
  const idx = Math.min(
    sorted.length - 1,
    Math.max(0, Math.ceil((p / 100) * sorted.length) - 1)
  );
  return sorted[idx];
}

function summarize(label, samples, errors, statuses) {
  const latencies = samples.map((row) => row.ms);
  const total = samples.length + errors.length;
  const statusCounts = {};
  for (const status of statuses) {
    statusCounts[String(status)] = (statusCounts[String(status)] || 0) + 1;
  }
  return {
    label,
    total,
    ok: samples.length,
    errors: errors.length,
    error_rate: total ? Number((errors.length / total).toFixed(4)) : 0,
    p50_ms: Number(percentile(latencies, 50).toFixed(2)),
    p95_ms: Number(percentile(latencies, 95).toFixed(2)),
    p99_ms: Number(percentile(latencies, 99).toFixed(2)),
    max_ms: Number((latencies.length ? Math.max(...latencies) : 0).toFixed(2)),
    status_counts: statusCounts,
    sample_error: errors[0] || null,
  };
}

async function hit(pathname, method = 'GET', body = null, timeoutMs = 8000) {
  const ctrl = new AbortController();
  const timer = setTimeout(() => ctrl.abort(new Error('timeout')), timeoutMs);
  const started = performance.now();
  try {
    const res = await fetch(`${BASE}${pathname}`, {
      method,
      headers: body ? { 'content-type': 'application/json' } : undefined,
      body: body ? JSON.stringify(body) : undefined,
      signal: ctrl.signal,
    });
    const text = await res.text();
    return {
      ok: res.ok,
      status: res.status,
      ms: performance.now() - started,
      text,
    };
  } finally {
    clearTimeout(timer);
  }
}

async function bombard({
  label,
  path,
  method = 'GET',
  body = null,
  requests = 200,
  concurrency = 20,
}) {
  const samples = [];
  const errors = [];
  const statuses = [];
  let cursor = 0;

  const worker = async () => {
    while (true) {
      const idx = cursor++;
      if (idx >= requests) break;
      try {
        const out = await hit(path, method, body);
        statuses.push(out.status);
        if (out.ok) {
          samples.push({ ms: out.ms });
        } else {
          errors.push(`HTTP ${out.status}`);
        }
      } catch (err) {
        errors.push(String(err && err.message ? err.message : err));
      }
      await sleep(0);
    }
  };

  await Promise.all(Array.from({ length: concurrency }, () => worker()));
  return summarize(label, samples, errors, statuses);
}

async function wsFanout({ connections = 36, timeoutMs = 4000 }) {
  const openTimes = [];
  const firstSnapshotTimes = [];
  const errors = [];

  await Promise.all(
    Array.from({ length: connections }, () =>
      new Promise((resolve) => {
        const started = performance.now();
        let openedAt = null;
        let settled = false;
        const ws = new WebSocket(`${WS_BASE}/ws`);

        const done = (cb) => {
          if (settled) return;
          settled = true;
          try {
            cb && cb();
          } catch {}
          resolve();
        };

        const timer = setTimeout(() => {
          errors.push('ws_timeout_first_snapshot');
          done(() => ws.terminate());
        }, timeoutMs);

        ws.on('open', () => {
          openedAt = performance.now();
          openTimes.push(openedAt - started);
        });

        ws.on('message', (raw) => {
          try {
            const msg = JSON.parse(String(raw || ''));
            if (msg && msg.type === 'snapshot') {
              firstSnapshotTimes.push(performance.now() - (openedAt || started));
              clearTimeout(timer);
              done(() => ws.close());
            }
          } catch {}
        });

        ws.on('error', (err) => {
          errors.push(`ws_error:${err && err.message ? err.message : 'unknown'}`);
          clearTimeout(timer);
          done(() => ws.terminate());
        });

        ws.on('close', () => {
          clearTimeout(timer);
          done();
        });
      })
    )
  );

  return {
    label: 'ws:/ws fanout',
    connections,
    ok: firstSnapshotTimes.length,
    errors: errors.length,
    error_rate: Number((errors.length / Math.max(1, connections)).toFixed(4)),
    open_latency: {
      p50_ms: Number(percentile(openTimes, 50).toFixed(2)),
      p95_ms: Number(percentile(openTimes, 95).toFixed(2)),
      p99_ms: Number(percentile(openTimes, 99).toFixed(2)),
    },
    first_snapshot_latency: {
      p50_ms: Number(percentile(firstSnapshotTimes, 50).toFixed(2)),
      p95_ms: Number(percentile(firstSnapshotTimes, 95).toFixed(2)),
      p99_ms: Number(percentile(firstSnapshotTimes, 99).toFixed(2)),
      max_ms: Number(
        (firstSnapshotTimes.length ? Math.max(...firstSnapshotTimes) : 0).toFixed(2)
      ),
    },
    sample_error: errors[0] || null,
  };
}

async function parsePostSnapshot() {
  const out = await hit('/api/dashboard/snapshot');
  if (!out.ok) return null;
  try {
    const parsed = JSON.parse(out.text || '{}');
    const runtime = parsed && parsed.runtime_recommendation ? parsed.runtime_recommendation : {};
    const attention = parsed && parsed.attention_queue ? parsed.attention_queue : {};
    const cockpit = parsed && parsed.cockpit ? parsed.cockpit : {};
    const health = parsed && parsed.health ? parsed.health : {};
    return {
      queue_depth:
        runtime.queue_depth != null
          ? runtime.queue_depth
          : attention.depth != null
            ? attention.depth
            : null,
      conduit_signals:
        runtime.conduit_signals != null
          ? runtime.conduit_signals
          : runtime.target_conduit_signals != null
            ? runtime.target_conduit_signals
            : null,
      cockpit_blocks:
        runtime.cockpit_blocks != null
          ? runtime.cockpit_blocks
          : Array.isArray(cockpit.active_blocks)
            ? cockpit.active_blocks.length
            : null,
      health_status_ok:
        health.ok != null
          ? health.ok
          : health.summary && health.summary.ok != null
            ? health.summary.ok
            : null,
    };
  } catch {
    return null;
  }
}

async function waitForHealthy(maxAttempts = 5) {
  let lastError = null;
  for (let attempt = 1; attempt <= maxAttempts; attempt += 1) {
    try {
      const out = await hit('/healthz', 'GET', null, 5000);
      if (out.ok) return true;
      lastError = new Error(`healthz_status_${out.status}`);
    } catch (err) {
      lastError = err;
    }
    await sleep(Math.min(4000, 500 * attempt));
  }
  throw lastError || new Error('healthz_unavailable');
}

async function main() {
  const started = performance.now();
  await waitForHealthy(6);

  const [health, status, snapshot, ws] = await Promise.all([
    bombard({ label: 'GET /healthz', path: '/healthz', requests: 160, concurrency: 20 }),
    bombard({ label: 'GET /api/status', path: '/api/status', requests: 320, concurrency: 32 }),
    bombard({
      label: 'GET /api/dashboard/snapshot',
      path: '/api/dashboard/snapshot',
      requests: 280,
      concurrency: 28,
    }),
    wsFanout({ connections: 36, timeoutMs: 4000 }),
  ]);

  const report = {
    type: 'infring_dashboard_stress_report',
    ts: new Date().toISOString(),
    target: BASE,
    duration_ms: Number((performance.now() - started).toFixed(2)),
    totals: {
      http_requests: health.total + status.total + snapshot.total,
      ws_connections: ws.connections,
    },
    endpoints: [health, status, snapshot, ws],
    post_snapshot: await parsePostSnapshot(),
  };

  process.stdout.write(`${JSON.stringify(report, null, 2)}\n`);

  const hasHttpErrors = [health, status, snapshot].some((row) => row.errors > 0);
  const hasWsErrors = ws.errors > 0;
  process.exitCode = hasHttpErrors || hasWsErrors ? 1 : 0;
}

main().catch((err) => {
  process.stderr.write(
    `dashboard_stress_smoke_failed: ${err && err.stack ? err.stack : String(err)}\n`
  );
  process.exit(2);
});
