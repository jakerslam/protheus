'use strict';

const assert = require('assert');
const fs = require('fs');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const HOST = process.env.INFRING_DASHBOARD_HOST || '127.0.0.1';
const BASE_PORT = Number(process.env.INFRING_DASHBOARD_PORT || 4340);
const PORT = Number.isFinite(BASE_PORT) && BASE_PORT > 0 ? BASE_PORT : 4340;
const BASE_URL = `http://${HOST}:${PORT}`;
const COLLAB_TEAM_STATE = path.resolve(ROOT, 'core/local/state/ops/collab_plane/teams/ops.json');

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function parseJson(text) {
  return JSON.parse(String(text || '').trim());
}

function authorityAgentShadows() {
  try {
    const parsed = JSON.parse(fs.readFileSync(COLLAB_TEAM_STATE, 'utf8'));
    const agents = Array.isArray(parsed && parsed.agents) ? parsed.agents : [];
    return agents
      .map((row) => String(row && row.shadow ? row.shadow : '').trim())
      .filter(Boolean);
  } catch {
    return [];
  }
}

async function fetchJson(url, init = {}, timeoutMs = 15000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try {
    const response = await fetch(url, {
      ...init,
      signal: controller.signal,
      headers: {
        'content-type': 'application/json',
        ...(init.headers || {}),
      },
    });
    const bodyText = await response.text();
    let body;
    try {
      body = parseJson(bodyText);
    } catch {
      body = { raw: bodyText };
    }
    return { status: response.status, ok: response.ok, body };
  } finally {
    clearTimeout(timer);
  }
}

async function waitForHealth(baseUrl, timeoutMs = 120000) {
  const start = Date.now();
  let lastError = null;
  while (Date.now() - start < timeoutMs) {
    try {
      const health = await fetchJson(`${baseUrl}/healthz`, {}, 5000);
      if (health.ok && health.body && health.body.ok === true) return health.body;
      lastError = new Error(`health_status_${health.status}`);
    } catch (error) {
      lastError = error;
    }
    await sleep(400);
  }
  throw lastError || new Error('dashboard_health_timeout');
}

async function waitForCondition(check, timeoutMs = 15000, intervalMs = 200) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await check();
    if (last) return last;
    await sleep(intervalMs);
  }
  return null;
}

async function postAction(baseUrl, action, payload) {
  return fetchJson(
    `${baseUrl}/api/dashboard/action`,
    {
      method: 'POST',
      body: JSON.stringify({ action, payload }),
    },
    90000
  );
}

function startServer() {
  const args = [
    ENTRYPOINT,
    TARGET,
    'serve',
    `--host=${HOST}`,
    `--port=${PORT}`,
  ];
  const child = spawn(process.execPath, args, {
    cwd: ROOT,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  let logs = '';
  child.stdout.on('data', (chunk) => {
    logs += chunk.toString();
  });
  child.stderr.on('data', (chunk) => {
    logs += chunk.toString();
  });
  return { child, getLogs: () => logs };
}

async function stopServer(child) {
  if (!child || child.killed) return;
  child.kill('SIGTERM');
  await sleep(250);
  if (!child.killed) {
    child.kill('SIGKILL');
    await sleep(150);
  }
}

async function run() {
  const { child, getLogs } = startServer();
  const summary = {
    type: 'infring_dashboard_llm_runtime_e2e',
    base_url: BASE_URL,
    checks: {},
    evidence: {},
  };
  try {
    await waitForHealth(BASE_URL);

    const snapshot = await fetchJson(`${BASE_URL}/api/dashboard/snapshot`);
    assert.strictEqual(snapshot.status, 200, 'snapshot endpoint should return 200');
    const s = snapshot.body || {};
    summary.checks.snapshot_layers_present = !!(
      s.cockpit
      && s.attention_queue
      && s.memory
      && Array.isArray(s.memory.entries)
      && s.receipts
      && s.logs
      && s.app
      && s.collab
    );
    assert.strictEqual(summary.checks.snapshot_layers_present, true, 'snapshot should expose cockpit/attention/memory/receipts/logs/app/collab');
    summary.checks.attention_priority_split = !!(
      s.attention_queue
      && s.attention_queue.priority_counts
      && Number.isFinite(Number(s.attention_queue.priority_counts.critical))
      && Number.isFinite(Number(s.attention_queue.priority_counts.telemetry))
      && Number.isFinite(Number(s.attention_queue.priority_counts.standard))
      && Number.isFinite(Number(s.attention_queue.priority_counts.background))
      && Array.isArray(s.attention_queue.critical_events)
      && Array.isArray(s.attention_queue.critical_events_full)
      && Number.isFinite(Number(s.attention_queue.critical_total_count))
      && Array.isArray(s.attention_queue.telemetry_events)
      && Array.isArray(s.attention_queue.standard_events)
      && Array.isArray(s.attention_queue.background_events)
      && s.attention_queue.lane_weights
      && Number.isFinite(Number(s.attention_queue.lane_weights.critical))
      && Number.isFinite(Number(s.attention_queue.lane_weights.standard))
      && Number.isFinite(Number(s.attention_queue.lane_weights.background))
    );
    assert.strictEqual(summary.checks.attention_priority_split, true, 'attention queue should expose critical/standard/background tier split');
    summary.checks.backpressure_signal_present = !!(
      s.attention_queue
      && s.attention_queue.backpressure
      && typeof s.attention_queue.backpressure.sync_mode === 'string'
      && typeof s.attention_queue.backpressure.level === 'string'
      && Number.isFinite(Number(s.attention_queue.backpressure.cockpit_to_conduit_ratio))
    );
    assert.strictEqual(summary.checks.backpressure_signal_present, true, 'attention queue should expose backpressure signals');
    const depthForSyncMode = Number((s.attention_queue && s.attention_queue.queue_depth) || 0);
    const expectedSyncMode = depthForSyncMode >= 75 ? 'batch_sync' : depthForSyncMode >= 50 ? 'delta_sync' : 'live_sync';
    summary.checks.backpressure_mode_consistent = String(s.attention_queue.backpressure.sync_mode) === expectedSyncMode;
    assert.strictEqual(summary.checks.backpressure_mode_consistent, true, 'sync mode should follow queue depth threshold policy');
    summary.checks.backpressure_lane_policy_present = !!(
      s.attention_queue
      && s.attention_queue.backpressure
      && s.attention_queue.backpressure.lane_weights
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_weights.critical))
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_weights.standard))
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_weights.background))
      && s.attention_queue.backpressure.lane_caps
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_caps.critical))
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_caps.standard))
      && Number.isFinite(Number(s.attention_queue.backpressure.lane_caps.background))
      && typeof s.attention_queue.backpressure.priority_preempt === 'boolean'
    );
    assert.strictEqual(summary.checks.backpressure_lane_policy_present, true, 'attention queue should expose lane weights/caps and preemption state');
    const queueDepth = Number((s.attention_queue && s.attention_queue.queue_depth) || 0);
    const minConduitTarget = queueDepth >= 65 ? 12 : 4;
    summary.checks.conduit_scale_target_present = !!(
      s.attention_queue
      && s.attention_queue.backpressure
      && Number.isFinite(Number(s.attention_queue.backpressure.target_conduit_signals))
      && Number(s.attention_queue.backpressure.target_conduit_signals) >= minConduitTarget
      && typeof s.attention_queue.backpressure.scale_required === 'boolean'
    );
    assert.strictEqual(summary.checks.conduit_scale_target_present, true, 'backpressure should include conduit scale target');
    summary.checks.cockpit_metrics_present = !!(
      s.cockpit
      && s.cockpit.metrics
      && s.cockpit.metrics.duration_ms
      && Number.isFinite(Number(s.cockpit.metrics.duration_ms.avg))
      && Number.isFinite(Number(s.cockpit.metrics.duration_ms.p95))
      && Number.isFinite(Number(s.cockpit.metrics.duration_ms.max))
      && s.cockpit.metrics.status_counts
      && s.cockpit.metrics.lane_counts
      && Array.isArray(s.cockpit.metrics.slowest_blocks)
      && Array.isArray(s.cockpit.trend)
    );
    assert.strictEqual(summary.checks.cockpit_metrics_present, true, 'cockpit metrics and trend should be present');
    summary.checks.cockpit_live_vs_total_present = !!(
      s.cockpit
      && Number.isFinite(Number(s.cockpit.block_count))
      && Number.isFinite(Number(s.cockpit.total_block_count))
      && Number.isFinite(Number(s.cockpit.metrics && s.cockpit.metrics.active_block_count))
      && Number.isFinite(Number(s.cockpit.metrics && s.cockpit.metrics.total_block_count))
      && Number(s.cockpit.total_block_count) >= Number(s.cockpit.block_count)
    );
    assert.strictEqual(
      summary.checks.cockpit_live_vs_total_present,
      true,
      'cockpit should expose live(active) and total block counts for stale-lock isolation'
    );
    const staleActionable = Number((s.cockpit && s.cockpit.metrics && s.cockpit.metrics.stale_block_count) || 0);
    const staleRaw = Number((s.cockpit && s.cockpit.metrics && s.cockpit.metrics.stale_block_raw_count) || 0);
    const staleDormant = Number((s.cockpit && s.cockpit.metrics && s.cockpit.metrics.stale_block_dormant_count) || 0);
    summary.checks.cockpit_stale_partition_present = !!(
      s.cockpit
      && s.cockpit.metrics
      && Number.isFinite(staleActionable)
      && Number.isFinite(staleRaw)
      && Number.isFinite(staleDormant)
      && staleRaw >= staleActionable
      && staleRaw >= staleDormant
      && Array.isArray(s.cockpit.metrics.stale_lanes_top)
      && Array.isArray(s.cockpit.metrics.stale_lanes_dormant_top)
    );
    assert.strictEqual(
      summary.checks.cockpit_stale_partition_present,
      true,
      'cockpit stale metrics should partition actionable and dormant stale blocks'
    );
    summary.checks.memory_stream_present = !!(
      s.memory
      && s.memory.stream
      && typeof s.memory.stream.enabled === 'boolean'
      && typeof s.memory.stream.changed === 'boolean'
      && Number.isFinite(Number(s.memory.stream.seq))
      && String(s.memory.stream.index_strategy || '') === 'hour_bucket_time_series'
    );
    assert.strictEqual(summary.checks.memory_stream_present, true, 'memory stream should expose hour-bucket time-series index metadata');
    summary.checks.memory_ingest_control_present = !!(
      s.memory
      && s.memory.ingest_control
      && typeof s.memory.ingest_control.paused === 'boolean'
      && Number.isFinite(Number(s.memory.ingest_control.pause_threshold))
      && Number.isFinite(Number(s.memory.ingest_control.resume_threshold))
      && Number.isFinite(Number(s.memory.ingest_control.memory_entry_threshold))
      && Number(s.memory.ingest_control.pause_threshold) === 80
      && Number(s.memory.ingest_control.resume_threshold) === 50
      && Number(s.memory.ingest_control.memory_entry_threshold) === 25
    );
    assert.strictEqual(summary.checks.memory_ingest_control_present, true, 'memory ingest control should expose queue+entry pressure thresholds');
    summary.checks.benchmark_sanity_health_present = !!(
      s.health
      && s.health.checks
      && s.health.checks.benchmark_sanity
      && typeof s.health.checks.benchmark_sanity.status === 'string'
    );
    assert.strictEqual(summary.checks.benchmark_sanity_health_present, true, 'health should expose benchmark_sanity check');
    summary.checks.health_coverage_present = !!(
      s.health
      && s.health.coverage
      && Number.isFinite(Number(s.health.coverage.count))
      && Number.isFinite(Number(s.health.coverage.previous_count))
      && Number.isFinite(Number(s.health.coverage.gap_count))
    );
    assert.strictEqual(summary.checks.health_coverage_present, true, 'health should expose coverage delta');
    summary.checks.agent_lifecycle_surface_present = !!(
      s.agent_lifecycle
      && Number.isFinite(Number(s.agent_lifecycle.active_count))
      && Number.isFinite(Number(s.agent_lifecycle.idle_agents))
      && Number.isFinite(Number(s.agent_lifecycle.idle_threshold))
      && typeof s.agent_lifecycle.idle_alert === 'boolean'
      && Array.isArray(s.agent_lifecycle.terminated_recent)
    );
    assert.strictEqual(
      summary.checks.agent_lifecycle_surface_present,
      true,
      'snapshot should expose agent lifecycle telemetry'
    );
    summary.checks.runtime_autoheal_surface_present = !!(
      s.runtime_autoheal
      && typeof s.runtime_autoheal.last_result === 'string'
      && typeof s.runtime_autoheal.last_stage === 'string'
      && typeof s.runtime_autoheal.stall_detected === 'boolean'
      && s.runtime_autoheal.cadence_ms
      && Number.isFinite(Number(s.runtime_autoheal.cadence_ms.normal))
      && Number.isFinite(Number(s.runtime_autoheal.cadence_ms.emergency))
    );
    assert.strictEqual(
      summary.checks.runtime_autoheal_surface_present,
      true,
      'snapshot should expose runtime autoheal telemetry'
    );
    summary.evidence.snapshot = {
      queue_depth: Number((s.attention_queue && s.attention_queue.queue_depth) || 0),
      cockpit_blocks: Number((s.cockpit && s.cockpit.block_count) || 0),
      cockpit_total_blocks: Number((s.cockpit && s.cockpit.total_block_count) || 0),
      memory_entries: Array.isArray(s.memory && s.memory.entries) ? s.memory.entries.length : 0,
      receipt_count: Array.isArray(s.receipts && s.receipts.recent) ? s.receipts.recent.length : 0,
      log_count: Array.isArray(s.logs && s.logs.recent) ? s.logs.recent.length : 0,
      sync_mode: String((s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.sync_mode) || ''),
      backpressure_level: String((s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.level) || ''),
      target_conduit_signals: Number((s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.target_conduit_signals) || 0),
      conduit_scale_required: !!(s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.scale_required),
      conduit_signals_effective: Number((s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.conduit_signals) || 0),
      conduit_signals_raw: Number((s.attention_queue && s.attention_queue.backpressure && s.attention_queue.backpressure.conduit_signals_raw) || 0),
      critical_attention: Number((s.attention_queue && s.attention_queue.priority_counts && s.attention_queue.priority_counts.critical) || 0),
      critical_attention_total: Number((s.attention_queue && s.attention_queue.critical_total_count) || 0),
      standard_attention: Number((s.attention_queue && s.attention_queue.priority_counts && s.attention_queue.priority_counts.standard) || 0),
      background_attention: Number((s.attention_queue && s.attention_queue.priority_counts && s.attention_queue.priority_counts.background) || 0),
      telemetry_micro_batches: Array.isArray(s.attention_queue && s.attention_queue.telemetry_micro_batches)
        ? s.attention_queue.telemetry_micro_batches.length
        : 0,
      lane_caps: s.attention_queue && s.attention_queue.backpressure ? s.attention_queue.backpressure.lane_caps : null,
      conduit_channels_observed: Number((s.cockpit && s.cockpit.metrics && s.cockpit.metrics.conduit_channels_observed) || 0),
      benchmark_sanity_status: String((s.health && s.health.checks && s.health.checks.benchmark_sanity && s.health.checks.benchmark_sanity.status) || ''),
      health_coverage_gap_count: Number((s.health && s.health.coverage && s.health.coverage.gap_count) || 0),
      memory_ingest_paused: !!(s.memory && s.memory.ingest_control && s.memory.ingest_control.paused),
    };

    const telemetry = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: 'Report runtime sync now: queue depth, cockpit blocks, conduit signals, and whether attention queue is readable.',
      }
    );
    assert.strictEqual(telemetry.status, 200, 'telemetry chat action should return 200');
    const telemetryLane = telemetry.body && telemetry.body.lane ? telemetry.body.lane : {};
    const telemetrySync = telemetryLane.runtime_sync || null;
    const telemetryResponseText = String(telemetryLane.response || '');
    const telemetryNumbersMatch = telemetryResponseText.match(
      /queue depth:\s*(\d+),\s*cockpit blocks:\s*(\d+)\s*active\s*\((\d+)\s*total\),\s*conduit signals:\s*(\d+)/i
    );
    const telemetryQueueFromText = telemetryNumbersMatch ? Number(telemetryNumbersMatch[1]) : null;
    const telemetryCockpitFromText = telemetryNumbersMatch ? Number(telemetryNumbersMatch[2]) : null;
    const telemetryCockpitTotalFromText = telemetryNumbersMatch ? Number(telemetryNumbersMatch[3]) : null;
    const telemetryConduitFromText = telemetryNumbersMatch ? Number(telemetryNumbersMatch[4]) : null;
    summary.checks.telemetry_runtime_sync = !!(telemetry.body && telemetry.body.ok && telemetrySync);
    summary.checks.telemetry_mentions_conduit = /conduit/i.test(String(telemetryLane.response || ''));
    summary.checks.telemetry_latency_fields_present = !!(
      telemetrySync
      && Object.prototype.hasOwnProperty.call(telemetrySync, 'receipt_latency_p95_ms')
      && Object.prototype.hasOwnProperty.call(telemetrySync, 'receipt_latency_p99_ms')
    );
    assert.strictEqual(summary.checks.telemetry_runtime_sync, true, 'telemetry response should include runtime_sync');
    assert.strictEqual(summary.checks.telemetry_mentions_conduit, true, 'telemetry response should mention conduit');
    assert.strictEqual(
      summary.checks.telemetry_latency_fields_present,
      true,
      'telemetry runtime sync should expose receipt latency SLO fields'
    );
    summary.checks.telemetry_response_runtime_sync_coherent = !!(
      telemetrySync
      && telemetryQueueFromText != null
      && telemetryCockpitFromText != null
      && telemetryCockpitTotalFromText != null
      && telemetryConduitFromText != null
      && Number(telemetrySync.queue_depth) === telemetryQueueFromText
      && Number(telemetrySync.cockpit_blocks) === telemetryCockpitFromText
      && Number(telemetrySync.cockpit_total_blocks) === telemetryCockpitTotalFromText
      && Number(telemetrySync.conduit_signals) === telemetryConduitFromText
    );
    assert.strictEqual(
      summary.checks.telemetry_response_runtime_sync_coherent,
      true,
      'telemetry response text and runtime_sync payload should report identical queue/cockpit/conduit values'
    );
    summary.evidence.telemetry = {
      lane_type: telemetryLane.type || '',
      queue_depth: telemetrySync ? telemetrySync.queue_depth : null,
      cockpit_blocks: telemetrySync ? telemetrySync.cockpit_blocks : null,
      cockpit_total_blocks: telemetrySync ? telemetrySync.cockpit_total_blocks : null,
      conduit_signals: telemetrySync ? telemetrySync.conduit_signals : null,
      conduit_signals_raw: telemetrySync ? telemetrySync.conduit_signals_raw : null,
      sync_mode: telemetrySync ? telemetrySync.sync_mode : null,
      backpressure_level: telemetrySync ? telemetrySync.backpressure_level : null,
      target_conduit_signals: telemetrySync ? telemetrySync.target_conduit_signals : null,
      conduit_scale_required: telemetrySync ? telemetrySync.conduit_scale_required : null,
      critical_attention_total: telemetrySync ? telemetrySync.critical_attention_total : null,
      benchmark_sanity_status: telemetrySync ? telemetrySync.benchmark_sanity_status : null,
      parsed_from_response: {
        queue_depth: telemetryQueueFromText,
        cockpit_blocks: telemetryCockpitFromText,
        cockpit_total_blocks: telemetryCockpitTotalFromText,
        conduit_signals: telemetryConduitFromText,
      },
      response_excerpt: telemetryResponseText.slice(0, 240),
    };

    const memory = await postAction(
      BASE_URL,
      'app.chat',
