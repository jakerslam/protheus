'use strict';

const assert = require('assert');
const http = require('http');
const path = require('path');
const { spawn } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const ENTRYPOINT = path.resolve(ROOT, 'client/runtime/lib/ts_entrypoint.ts');
const TARGET = path.resolve(ROOT, 'client/runtime/systems/ui/infring_dashboard.ts');
const HOST = '127.0.0.1';
const PORT = Number(process.env.INFRING_DASHBOARD_NATIVE_E2E_PORT || 4384);
const API_PORT = Number(process.env.INFRING_DASHBOARD_NATIVE_E2E_API_PORT || 5384);
const BASE_URL = `http://${HOST}:${PORT}`;

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function fetchJson(url, init = {}, timeoutMs = 10000) {
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
    const text = await response.text();
    let body = { raw: text };
    try {
      body = JSON.parse(String(text || '').trim());
    } catch {}
    return { status: response.status, ok: response.ok, body };
  } finally {
    clearTimeout(timer);
  }
}

async function waitFor(check, timeoutMs = 15000, intervalMs = 200) {
  const started = Date.now();
  let last = null;
  while (Date.now() - started < timeoutMs) {
    last = await check();
    if (last) return last;
    await sleep(intervalMs);
  }
  return last;
}

function sendJson(res, value, status = 200) {
  res.writeHead(status, { 'content-type': 'application/json; charset=utf-8' });
  res.end(`${JSON.stringify(value)}\n`);
}

function readJsonBody(req) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)));
    req.on('end', () => {
      if (!chunks.length) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch (error) {
        reject(error);
      }
    });
    req.on('error', reject);
  });
}

function startStubBackend() {
  const state = {
    nextAgent: 2,
    agents: [
      { id: 'agent-stub', name: 'Stub Agent', status: 'ready' },
    ],
    sessions: {
      'agent-stub': [
        { id: 'msg-stub-1', role: 'assistant', text: 'stub hello', created_at: '2026-04-13T20:00:00.000Z' },
      ],
    },
  };
  const server = http.createServer((req, res) => {
    const url = new URL(req.url || '/', `http://${HOST}:${API_PORT}`);
    const pathname = url.pathname;

    if (req.method === 'GET' && pathname === '/healthz') {
      sendJson(res, { ok: true, backend: 'stub' });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/status') {
      sendJson(res, {
        ok: true,
        connected: true,
        uptime_seconds: 864,
        default_provider: 'openai',
        default_model: 'gpt-5.4',
        api_listen: `http://${HOST}:${API_PORT}`,
        home_dir: ROOT,
        log_level: 'info',
        network_enabled: true,
        agent_count: 1,
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/version') {
      sendJson(res, {
        ok: true,
        version: '1.2.3-stub',
        platform: process.platform,
        arch: process.arch,
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/providers') {
      sendJson(res, {
        providers: [
          {
            id: 'openai',
            display_name: 'OpenAI',
            auth_status: 'Configured',
            api_key_env: 'OPENAI_API_KEY',
            base_url: 'https://api.openai.com/v1',
            is_local: false,
            reachable: true,
          },
        ],
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/agents') {
      sendJson(res, state.agents);
      return;
    }
    if (req.method === 'POST' && pathname === '/api/agents') {
      readJsonBody(req)
        .then((body) => {
          const id = `agent-stub-${state.nextAgent++}`;
          const name = String(body && body.name ? body.name : id).trim() || id;
          const row = { id, name, status: 'ready' };
          state.agents.push(row);
          state.sessions[id] = [];
          sendJson(res, row);
        })
        .catch(() => sendJson(res, { ok: false, error: 'invalid_json' }, 400));
      return;
    }
    const sessionMatch = pathname.match(/^\/api\/agents\/([^/]+)\/session$/);
    if (req.method === 'GET' && sessionMatch) {
      const agentId = decodeURIComponent(sessionMatch[1]);
      sendJson(res, {
        session_id: `session-${agentId}`,
        messages: Array.isArray(state.sessions[agentId]) ? state.sessions[agentId] : [],
      });
      return;
    }
    const messageMatch = pathname.match(/^\/api\/agents\/([^/]+)\/message$/);
    if (req.method === 'POST' && messageMatch) {
      const agentId = decodeURIComponent(messageMatch[1]);
      readJsonBody(req)
        .then((body) => {
          const prompt = String(body && body.message ? body.message : '').trim() || 'hello';
          const session = Array.isArray(state.sessions[agentId]) ? state.sessions[agentId] : [];
          const userMessage = {
            id: `msg-user-${session.length + 1}`,
            role: 'user',
            text: prompt,
            created_at: '2026-04-13T20:05:00.000Z',
          };
          const assistantMessage = {
            id: `msg-assistant-${session.length + 2}`,
            role: 'assistant',
            text: `stub reply for ${agentId}`,
            created_at: '2026-04-13T20:05:01.000Z',
          };
          session.push(userMessage, assistantMessage);
          state.sessions[agentId] = session;
          sendJson(res, {
            ok: true,
            agent_id: agentId,
            user_message_id: userMessage.id,
            message_id: assistantMessage.id,
            reply: assistantMessage.text,
          });
        })
        .catch(() => sendJson(res, { ok: false, error: 'invalid_json' }, 400));
      return;
    }
    if (req.method === 'GET' && pathname === '/api/models') {
      sendJson(res, {
        models: [
          {
            id: 'gpt-5.4',
            provider: 'openai',
            display_name: 'GPT-5.4',
            local: false,
          },
        ],
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/skills') {
      sendJson(res, {
        skills: [
          {
            name: 'browser',
            description: 'Stub browser skill',
            version: '1.0.0',
            author: 'ops',
            runtime: 'prompt_only',
            tools_count: 2,
            enabled: true,
            tags: ['web', 'tooling'],
          },
        ],
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/mcp/servers') {
      sendJson(res, {
        configured: [{ id: 'github', label: 'GitHub' }],
        connected: [{ id: 'github', label: 'GitHub' }],
        total_configured: 1,
        total_connected: 1,
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/web/status') {
      sendJson(res, {
        enabled: true,
        receipts_total: 4,
        recent_denied: 1,
        last_receipt: { requested_url: 'https://example.com' },
        policy: {
          web_conduit: {
            rate_limit_per_minute: 12,
          },
        },
      });
      return;
    }
    if (req.method === 'GET' && pathname === '/api/web/receipts') {
      sendJson(res, {
        receipts: [
          {
            requested_url: 'https://example.com',
            method: 'GET',
            status: 'allowed',
            blocked: false,
            created_at: '2026-04-13T20:00:00.000Z',
          },
        ],
      });
      return;
    }

    sendJson(res, { ok: false, error: `stub_not_found:${pathname}` }, 404);
  });

  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(API_PORT, HOST, () => resolve(server));
  });
}

function startDashboard() {
  const child = spawn(
    process.execPath,
    [
      ENTRYPOINT,
      TARGET,
      'serve',
      `--host=${HOST}`,
      `--port=${PORT}`,
      `--api-host=${HOST}`,
      `--api-port=${API_PORT}`,
    ],
    {
      cwd: ROOT,
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    }
  );
  let logs = '';
  child.stdout.on('data', (chunk) => {
    logs += chunk.toString();
  });
  child.stderr.on('data', (chunk) => {
    logs += chunk.toString();
  });
  return { child, getLogs: () => logs };
}

async function stopChild(child) {
  if (!child || child.exitCode != null || child.killed) return;
  child.kill('SIGTERM');
  await sleep(250);
  if (child.exitCode == null && !child.killed) {
    child.kill('SIGKILL');
    await sleep(150);
  }
}

async function run() {
  const backend = await startStubBackend();
  const { child, getLogs } = startDashboard();
  const summary = {
    type: 'infring_dashboard_native_operator_flows',
    base_url: BASE_URL,
    checks: {},
    evidence: {},
  };

  try {
    const ready = await waitFor(async () => {
      try {
        const response = await fetchJson(`${BASE_URL}/api/status`);
        return response.ok ? response : null;
      } catch {
        return null;
      }
    }, 15000, 250);
    assert.ok(ready && ready.ok, 'dashboard host should become ready against stub backend');

    const policyDebt = await fetchJson(`${BASE_URL}/api/runtime/policy-debt`);
    const orchestration = await fetchJson(`${BASE_URL}/api/runtime/orchestration-surface`);
    const agentsBefore = await fetchJson(`${BASE_URL}/api/agents`);
    const createdAgent = await fetchJson(`${BASE_URL}/api/agents`, {
      method: 'POST',
      body: JSON.stringify({ name: 'Native Flow Agent', role: 'assistant' }),
    });
    const createdAgentId = String(createdAgent.body && createdAgent.body.id ? createdAgent.body.id : '');
    const sessionBefore = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(createdAgentId)}/session`);
    const messageResult = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(createdAgentId)}/message`, {
      method: 'POST',
      body: JSON.stringify({ message: 'hello from native flow test' }),
    });
    const sessionAfter = await fetchJson(`${BASE_URL}/api/agents/${encodeURIComponent(createdAgentId)}/session`);
    const providers = await fetchJson(`${BASE_URL}/api/providers`);
    const models = await fetchJson(`${BASE_URL}/api/models`);
    const skills = await fetchJson(`${BASE_URL}/api/skills`);
    const mcp = await fetchJson(`${BASE_URL}/api/mcp/servers`);
    const webStatus = await fetchJson(`${BASE_URL}/api/web/status`);
    const webReceipts = await fetchJson(`${BASE_URL}/api/web/receipts?limit=5`);

    summary.checks.runtime_policy_debt_surface = Boolean(
      policyDebt.status === 200
      && policyDebt.body
      && policyDebt.body.ok
      && policyDebt.body.summary
      && Number(policyDebt.body.summary.classic_asset_files || 0) >= 1
      && Array.isArray(policyDebt.body.top_classic_files)
      && policyDebt.body.top_classic_files.length >= 1
    );
    summary.checks.chat_surface_roundtrip = Boolean(
      agentsBefore.status === 200
      && Array.isArray(agentsBefore.body)
      && createdAgent.status === 200
      && createdAgentId
      && sessionBefore.status === 200
      && Array.isArray(sessionBefore.body && sessionBefore.body.messages)
      && sessionBefore.body.messages.length === 0
      && messageResult.status === 200
      && messageResult.body
      && messageResult.body.ok === true
      && sessionAfter.status === 200
      && Array.isArray(sessionAfter.body && sessionAfter.body.messages)
      && sessionAfter.body.messages.length >= 2
      && sessionAfter.body.messages.some((row) => row && row.role === 'assistant')
    );
    summary.checks.runtime_orchestration_surface = Boolean(
      orchestration.status === 200
      && orchestration.body
      && orchestration.body.ok
      && orchestration.body.summary
      && orchestration.body.summary.capability_probes === true
      && orchestration.body.summary.alternative_plans === true
      && orchestration.body.summary.verifier_request === true
      && orchestration.body.summary.receipt_correlation === true
      && Array.isArray(orchestration.body.correlation_fields)
      && orchestration.body.correlation_fields.includes('orchestration_trace_id')
    );
    summary.checks.settings_provider_and_model_surfaces = Boolean(
      providers.status === 200
      && Array.isArray(providers.body && providers.body.providers)
      && models.status === 200
      && Array.isArray(models.body && models.body.models)
    );
    summary.checks.skills_and_mcp_surfaces = Boolean(
      skills.status === 200
      && Array.isArray(skills.body && skills.body.skills)
      && mcp.status === 200
      && Array.isArray(mcp.body && mcp.body.configured)
      && Array.isArray(mcp.body && mcp.body.connected)
      && Number.isFinite(Number(mcp.body && mcp.body.total_configured))
      && Number.isFinite(Number(mcp.body && mcp.body.total_connected))
    );
    summary.checks.runtime_web_tooling_surfaces = Boolean(
      webStatus.status === 200
      && webStatus.body
      && webStatus.body.enabled === true
      && webReceipts.status === 200
      && Array.isArray(webReceipts.body && webReceipts.body.receipts)
    );

    Object.entries(summary.checks).forEach(([label, value]) => {
      assert.strictEqual(value, true, `${label} should pass`);
    });

    summary.evidence = {
      policy_debt: policyDebt.body,
      orchestration_surface: orchestration.body,
      chat_flow: {
        agents_before: agentsBefore.body,
        created_agent: createdAgent.body,
        session_before: sessionBefore.body,
        message_result: messageResult.body,
        session_after: sessionAfter.body,
      },
      providers: providers.body,
      models: models.body,
      skills: skills.body,
      mcp: mcp.body,
      web_status: webStatus.body,
      web_receipts: webReceipts.body,
    };
    summary.ok = true;
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
    await stopChild(child);
    await new Promise((resolve) => backend.close(() => resolve(null)));
  }
}

run().catch(() => {
  process.exitCode = 1;
});
