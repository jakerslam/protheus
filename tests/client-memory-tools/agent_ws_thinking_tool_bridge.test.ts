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

function structuredContentPayload() {
  return {
    ok: true,
    response: '',
    content: [
      {
        type: 'tool_call',
        id: 'call-fetch-1',
        name: 'web_fetch',
        arguments: { url: 'https://example.com' },
      },
      {
        type: 'tool_result',
        tool_use_id: 'call-fetch-1',
        content: 'Example Domain summary',
      },
    ],
    response_finalization: {
      tool_completion: {
        live_tool_steps: [
          { tool: 'web_fetch', status: 'fetched https://example.com' },
        ],
        tool_attempts: [
          {
            attempt: {
              attempt_id: 'call-fetch-1',
              tool_name: 'web_fetch',
              status: 'ok',
              reason: 'ok',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { url: 'https://example.com' } },
          },
        ],
      },
    },
  };
}

function structuredErrorPayload() {
  return {
    ok: true,
    response: '',
    content: [
      {
        type: 'tool_call',
        id: 'call-search-1',
        name: 'web_search',
        arguments: { query: 'agent reliability benchmarks' },
      },
      {
        type: 'tool_result_error',
        tool_use_id: 'call-search-1',
        result: {
          content: [
            { type: 'text', text: 'provider timeout after 30s' },
          ],
        },
        details: { status: 'timeout', reason: 'provider_timeout' },
      },
    ],
    response_finalization: {
      tool_completion: {
        live_tool_steps: [
          { tool: 'web_search', status: 'provider timeout after 30s' },
        ],
        tool_attempts: [
          {
            attempt: {
              attempt_id: 'call-search-1',
              tool_name: 'web_search',
              status: 'timeout',
              reason: 'provider_timeout',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { query: 'agent reliability benchmarks' } },
          },
        ],
      },
    },
  };
}

function structuredLargeToolResultPayload() {
  return {
    ok: true,
    response: '',
    content: [
      {
        type: 'tool_call',
        id: 'call-batch-1',
        name: 'batch_query',
        arguments: { query: 'compare openclaw to this workspace' },
      },
      {
        type: 'tool_result',
        tool_use_id: 'call-batch-1',
        content: 'alpha line\n'.repeat(700) + '\nsummary: final ranked comparison ready\nerror tail preserved',
      },
    ],
    context_window: 4096,
    response_finalization: {
      tool_completion: {
        live_tool_steps: [
          { tool: 'batch_query', status: 'completed ranked comparison retrieval' },
        ],
        tool_attempts: [
          {
            attempt: {
              attempt_id: 'call-batch-1',
              tool_name: 'batch_query',
              status: 'ok',
              reason: 'ok',
              backend: 'retrieval_plane',
            },
            normalized_result: { normalized_args: { query: 'compare openclaw to this workspace' } },
          },
        ],
      },
    },
  };
}

async function runScenario(payloadFactory, messageText) {
  const flags = { host: '127.0.0.1', port: 0 };
  const fetchBackendJson = async (_flags, route) => {
    if (String(route || '').includes('/api/agents/agent-1')) {
      return { name: 'Probe Spark', context_window: 262144 };
    }
    return {};
  };
  const fetchBackend = async (_flags, route) => {
    if (String(route || '').includes('/api/agents/agent-1/message')) {
      return { ok: true, status: 200, json: async () => payloadFactory() };
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
        ws.send(JSON.stringify({ type: 'message', content: messageText }));
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
  return { response, events };
}

async function run() {
  const scenarioOne = await runScenario(completionPayload, 'run two searches');
  const response = scenarioOne.response;
  const events = scenarioOne.events;

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

  const scenarioTwo = await runScenario(structuredContentPayload, 'run tool blocks');
  const structuredResponse = scenarioTwo.response;
  const structuredEvents = scenarioTwo.events;
  const structuredTools = Array.isArray(structuredResponse.tools) ? structuredResponse.tools : [];
  assert.strictEqual(
    String(structuredResponse.content || ''),
    '',
    'structured tool-only completions may have empty prose content'
  );
  assert.strictEqual(structuredTools.length, 1, 'structured tool blocks should become response tool rows');
  assert.strictEqual(structuredTools[0].attempt_id, 'call-fetch-1', 'structured tool rows should preserve tool use ids');
  assert.strictEqual(structuredTools[0].name, 'web_fetch', 'structured tool rows should preserve tool names');
  assert.strictEqual(structuredTools[0].result, 'Example Domain summary', 'structured tool rows should preserve tool results');
  const structuredToolResult = structuredEvents.find((row) => row.type === 'tool_result');
  assert.ok(structuredToolResult, 'structured tool-only completion should still replay tool_result events');
  assert.strictEqual(structuredToolResult.attempt_id, 'call-fetch-1');

  const scenarioThree = await runScenario(structuredErrorPayload, 'run failing tool blocks');
  const structuredErrorTools = Array.isArray(scenarioThree.response.tools) ? scenarioThree.response.tools : [];
  assert.strictEqual(structuredErrorTools.length, 1, 'structured tool error blocks should become response tool rows');
  assert.strictEqual(structuredErrorTools[0].attempt_id, 'call-search-1');
  assert.strictEqual(structuredErrorTools[0].is_error, true, 'structured tool error block should stay marked as error');
  assert.strictEqual(structuredErrorTools[0].status, 'timeout', 'structured tool error block should preserve timeout status');
  assert.strictEqual(
    structuredErrorTools[0].result,
    'provider timeout after 30s',
    'structured tool error block should preserve nested text results instead of JSON blobs'
  );

  const scenarioFour = await runScenario(structuredLargeToolResultPayload, 'run large tool blocks');
  const largeTools = Array.isArray(scenarioFour.response.tools) ? scenarioFour.response.tools : [];
  assert.strictEqual(largeTools.length, 1, 'large structured tool result should stay visible');
  assert.ok(
    largeTools[0].result.includes('more characters truncated'),
    'large structured tool results should be truncated with an OpenClaw-style notice'
  );
  assert.ok(
    largeTools[0].result.includes('summary: final ranked comparison ready'),
    'truncation should preserve important tail content'
  );
}

run().catch((error) => {
  console.error(error && error.stack ? error.stack : error);
  process.exit(1);
});
