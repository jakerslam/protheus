'use strict';

const assert = require('assert');
const http = require('http');
const path = require('path');
const WebSocket = require('ws');

const ROOT = path.resolve(__dirname, '..', '..');
const { createAgentWsBridge } = require(
  path.resolve(ROOT, 'client/runtime/systems/ui/agent_ws_bridge.ts')
);

function completionPayload() {
  return {
    ok: true,
    response: 'done',
    tools: [
      { name: 'web_search', input: '{"query":"alpha"}', result: 'alpha result', status: 'ok' },
      { name: 'web_search', input: '{"query":"beta"}', result: 'beta result', status: 'ok' },
      { name: 'file_search', input: '{"query":"secret.txt"}', result: 'policy denied by runtime', status: 'blocked', blocked: true },
      { name: 'web_search', input: '{"query":"gamma"}', result: 'request_read_failed', status: 'error', is_error: true },
    ],
    response_finalization: {
      tool_completion: {
        live_tool_steps: [
          { tool: 'web_search', status: 'searched for alpha' },
          { tool: 'web_search', status: 'searched for beta' },
          { tool: 'file_search', status: 'blocked by policy' },
          { tool: 'web_search', status: 'request read failed; suggest narrower query' },
        ],
        tool_attempts: [
          {
            attempt: {
              attempt_id: 'attempt-alpha',
              tool_name: 'web_search',
              status: 'ok',
              reason: 'ok',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { query: 'alpha' } },
          },
          {
            attempt: {
              attempt_id: 'attempt-beta',
              tool_name: 'web_search',
              status: 'ok',
              reason: 'ok',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { query: 'beta' } },
          },
          {
            attempt: {
              attempt_id: 'attempt-blocked',
              tool_name: 'file_search',
              status: 'blocked',
              reason: 'policy_denied',
              backend: 'workspace_plane',
            },
            normalized_result: { normalized_args: { query: 'secret.txt' } },
          },
          {
            attempt: {
              attempt_id: 'attempt-error',
              tool_name: 'web_search',
              status: 'error',
              reason: 'request_read_failed',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { query: 'gamma' } },
          },
        ],
      },
    },
  };
}

async function run() {
  const flags = { host: '127.0.0.1', port: 0 };
  const fetchBackendJson = async (_flags, route) => {
    if (String(route || '').includes('/api/agents/agent-1')) {
      return { name: 'Probe Spark', context_window: 262144 };
    }
    return {};
  };
  const fetchBackend = async (_flags, route) => {
    if (String(route || '').includes('/api/agents/agent-1/message')) {
      return { ok: true, status: 200, json: async () => completionPayload() };
    }
    return { ok: false, status: 404, json: async () => ({ error: 'not_found' }) };
  };
  const bridge = createAgentWsBridge({
    flags,
    cleanText: (value, max) => String(value == null ? '' : value).trim().slice(0, max || 200),
    fetchBackend,
    fetchBackendJson,
  });
  assert.ok(bridge && bridge.ws_enabled, 'agent ws bridge should initialize with ws available');
  const server = http.createServer((_req, res) => {
    res.writeHead(404);
    res.end('not-found');
  });
  server.on('upgrade', (req, socket, head) => {
    if (!bridge.tryHandle(req, socket, head)) {
      socket.destroy();
    }
  });
  await new Promise((resolve) => server.listen(0, '127.0.0.1', resolve));
  const { port } = server.address();
  const ws = new WebSocket(`ws://127.0.0.1:${port}/api/agents/agent-1/ws`);
  const events = [];
  const responsePromise = new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error('ws_response_timeout')), 10000);
    ws.on('message', (chunk) => {
      const parsed = JSON.parse(String(chunk || '{}'));
      events.push(parsed);
      if (parsed.type === 'connected') {
        ws.send(JSON.stringify({ type: 'message', content: 'run two searches' }));
      }
      if (parsed.type === 'response') {
        clearTimeout(timeout);
        resolve(parsed);
      }
    });
    ws.on('error', (error) => {
      clearTimeout(timeout);
      reject(error);
    });
  });
  const response = await responsePromise;
  ws.close();
  server.close();

  const responseTools = Array.isArray(response.tools) ? response.tools : [];
  assert.strictEqual(responseTools.length, 4, 'response payload should preserve repeated, blocked, and failed attempts');
  assert.deepStrictEqual(
    responseTools.map((row) => row.attempt_id),
    ['attempt-alpha', 'attempt-beta', 'attempt-blocked', 'attempt-error'],
    'response tool rows should key off authoritative attempt ids across repeated and failed tool calls'
  );
  const byAttempt = new Map(responseTools.map((row) => [row.attempt_id, row]));
  assert.strictEqual(byAttempt.get('attempt-blocked').blocked, true, 'blocked attempts should remain visible');
  assert.strictEqual(byAttempt.get('attempt-blocked').status, 'blocked', 'blocked attempts should keep blocked status');
  assert.strictEqual(byAttempt.get('attempt-error').is_error, true, 'errored attempts should remain visible');
  assert.strictEqual(byAttempt.get('attempt-error').status, 'error', 'errored attempts should keep error status');
  const toolStartEvents = events.filter((row) => row.type === 'tool_start');
  assert.strictEqual(toolStartEvents.length, 4, 'ws runtime should replay every tool_start event');
  assert.deepStrictEqual(
    toolStartEvents.map((row) => row.attempt_id),
    ['attempt-alpha', 'attempt-beta', 'attempt-blocked', 'attempt-error'],
    'tool lifecycle events should carry unique attempt ids for repeated, blocked, and failed tool calls'
  );
  const toolResultEvents = events.filter((row) => row.type === 'tool_result');
  assert.strictEqual(toolResultEvents.length, 4, 'ws runtime should replay every tool_result event');
  const blockedEvent = toolResultEvents.find((row) => row.attempt_id === 'attempt-blocked');
  assert.ok(blockedEvent, 'blocked tool result event should be present');
  assert.strictEqual(blockedEvent.tool_status, 'blocked by policy', 'blocked tool result should carry completion status');
  const errorEvent = toolResultEvents.find((row) => row.attempt_id === 'attempt-error');
  assert.ok(errorEvent, 'failed tool result event should be present');
  assert.strictEqual(errorEvent.is_error, true, 'failed tool result should remain flagged as error');
}

run().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});
