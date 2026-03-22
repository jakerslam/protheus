#!/usr/bin/env node
'use strict';

const assert = require('assert');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const HOST = process.env.INFRING_DASHBOARD_HOST || '127.0.0.1';
const BASE_PORT = Number(process.env.INFRING_DASHBOARD_PORT || 4340);
const PORT = Number.isFinite(BASE_PORT) && BASE_PORT > 0 ? BASE_PORT : 4340;
const BASE_URL = `http://${HOST}:${PORT}`;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function parseJson(text) {
  return JSON.parse(String(text || '').trim());
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

async function postAction(baseUrl, action, payload) {
  return fetchJson(
    `${baseUrl}/api/dashboard/action`,
    {
      method: 'POST',
      body: JSON.stringify({ action, payload }),
    },
    30000
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
    summary.evidence.snapshot = {
      queue_depth: Number((s.attention_queue && s.attention_queue.queue_depth) || 0),
      cockpit_blocks: Number((s.cockpit && s.cockpit.block_count) || 0),
      memory_entries: Array.isArray(s.memory && s.memory.entries) ? s.memory.entries.length : 0,
      receipt_count: Array.isArray(s.receipts && s.receipts.recent) ? s.receipts.recent.length : 0,
      log_count: Array.isArray(s.logs && s.logs.recent) ? s.logs.recent.length : 0,
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
    summary.checks.telemetry_runtime_sync = !!(telemetry.body && telemetry.body.ok && telemetrySync);
    summary.checks.telemetry_mentions_conduit = /conduit/i.test(String(telemetryLane.response || ''));
    assert.strictEqual(summary.checks.telemetry_runtime_sync, true, 'telemetry response should include runtime_sync');
    assert.strictEqual(summary.checks.telemetry_mentions_conduit, true, 'telemetry response should mention conduit');
    summary.evidence.telemetry = {
      lane_type: telemetryLane.type || '',
      queue_depth: telemetrySync ? telemetrySync.queue_depth : null,
      cockpit_blocks: telemetrySync ? telemetrySync.cockpit_blocks : null,
      conduit_signals: telemetrySync ? telemetrySync.conduit_signals : null,
      response_excerpt: String(telemetryLane.response || '').slice(0, 240),
    };

    const memory = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: 'What were we doing one week ago? Return exact date and memory file path.',
      }
    );
    assert.strictEqual(memory.status, 200, 'memory query should return 200');
    const memoryLane = memory.body && memory.body.lane ? memory.body.lane : {};
    const memoryText = String(memoryLane.response || '');
    const memoryTools = Array.isArray(memoryLane.tools) ? memoryLane.tools : [];
    summary.checks.week_ago_memory_recall = Boolean(
      memory.body
      && memory.body.ok
      && /\b20\d{2}-\d{2}-\d{2}\b/.test(memoryText)
      && /local\/workspace\/memory\//.test(memoryText)
    );
    summary.checks.week_ago_used_memory_tool = memoryTools.some((tool) =>
      String(tool && tool.input ? tool.input : '').includes('local/workspace/memory/')
    );
    assert.strictEqual(summary.checks.week_ago_memory_recall, true, 'week-ago response should include exact date and memory path');
    assert.strictEqual(summary.checks.week_ago_used_memory_tool, true, 'week-ago response should include memory file read tool evidence');
    summary.evidence.week_ago = {
      response_excerpt: memoryText.slice(0, 300),
      tools: memoryTools.map((tool) => String(tool && tool.input ? tool.input : '')).slice(0, 4),
    };

    const clientLayer = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: 'Summarize client layer now with memory entries, receipts, logs, health checks, attention queue, and cockpit.',
      }
    );
    assert.strictEqual(clientLayer.status, 200, 'client-layer query should return 200');
    const clientLane = clientLayer.body && clientLayer.body.lane ? clientLayer.body.lane : {};
    const clientText = String(clientLane.response || '');
    summary.checks.client_layer_visibility = Boolean(
      clientLayer.body
      && clientLayer.body.ok
      && clientText.trim().toLowerCase() !== '<text response to user>'
      && /memory|receipt|log|health|attention|cockpit/i.test(clientText)
    );
    assert.strictEqual(summary.checks.client_layer_visibility, true, 'client-layer response should expose runtime surfaces');
    summary.evidence.client_layer = {
      response_excerpt: clientText.slice(0, 300),
    };

    const suffix = String(Date.now()).slice(-6);
    const coordinatorShadow = `e2e-${suffix}-coord`;
    const researcherShadow = `e2e-${suffix}-res`;
    const swarm = await postAction(
      BASE_URL,
      'app.chat',
      {
        input: [
          'Run exactly these commands to create a swarm of subagents:',
          `protheus-ops collab-plane launch-role --team=ops --role=coordinator --shadow=${coordinatorShadow}`,
          `protheus-ops collab-plane launch-role --team=ops --role=researcher --shadow=${researcherShadow}`,
        ].join('\n'),
      }
    );
    assert.strictEqual(swarm.status, 200, 'swarm launch query should return 200');
    const swarmLane = swarm.body && swarm.body.lane ? swarm.body.lane : {};
    const swarmTools = Array.isArray(swarmLane.tools) ? swarmLane.tools : [];
    summary.checks.swarm_launch_commands_executed =
      swarmTools.filter((tool) => String(tool && tool.input ? tool.input : '').includes('collab-plane launch-role')).length >= 2;
    assert.strictEqual(summary.checks.swarm_launch_commands_executed, true, 'swarm action should execute launch-role commands');

    const snapshotAfter = await fetchJson(`${BASE_URL}/api/dashboard/snapshot`);
    assert.strictEqual(snapshotAfter.status, 200, 'snapshot-after endpoint should return 200');
    const collabString = JSON.stringify(
      snapshotAfter.body && snapshotAfter.body.collab && typeof snapshotAfter.body.collab === 'object'
        ? snapshotAfter.body.collab
        : {}
    );
    summary.checks.swarm_agents_visible_in_collab =
      collabString.includes(coordinatorShadow) && collabString.includes(researcherShadow);
    assert.strictEqual(summary.checks.swarm_agents_visible_in_collab, true, 'collab dashboard should contain newly created swarm shadows');
    summary.evidence.swarm = {
      lane_response: String(swarmLane.response || '').slice(0, 240),
      tool_inputs: swarmTools.map((tool) => String(tool && tool.input ? tool.input : '')).slice(0, 6),
      collab_contains: { coordinatorShadow, researcherShadow },
    };

    summary.ok = Object.values(summary.checks).every(Boolean);
    assert.strictEqual(summary.ok, true, 'all checks should pass');
    console.log(JSON.stringify(summary, null, 2));
  } catch (error) {
    const failure = {
      ...summary,
      ok: false,
      error: String(error && error.stack ? error.stack : error),
      logs_tail: getLogs().slice(-4000),
    };
    console.error(JSON.stringify(failure, null, 2));
    throw error;
  } finally {
    await stopServer(child);
  }
}

run().catch(() => {
  process.exitCode = 1;
});
