#!/usr/bin/env node
'use strict';

// Layer ownership: client/runtime/systems/autonomy (thin bridge over core/layer0/ops swarm-runtime).
// Purpose: compatibility surface for OpenClaw-style sessions_* swarm operations.

const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const OPS_WRAPPER = path.join(
  ROOT,
  'client',
  'runtime',
  'systems',
  'ops',
  'run_protheus_ops.js'
);
const DEFAULT_STATE_PATH = path.join(ROOT, 'local', 'state', 'ops', 'swarm_runtime', 'latest.json');

function parseArgs(argv) {
  const out = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const token = String(argv[i] || '');
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx >= 0) {
      out[token.slice(2, idx)] = token.slice(idx + 1);
      continue;
    }
    const key = token.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function normalizedOptions(options) {
  if (options && typeof options === 'object' && !Array.isArray(options)) return options;
  return {};
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function statePath(parsed) {
  const explicit = String(parsed['state-path'] || parsed.state_path || '').trim();
  return explicit || DEFAULT_STATE_PATH;
}

function asInt(value, fallback, min = 0) {
  const parsed = Number.parseInt(String(value == null ? '' : value), 10);
  if (!Number.isFinite(parsed)) return fallback;
  return Math.max(min, parsed);
}

function asFloat(value, fallback, min = 0, max = 1) {
  const parsed = Number.parseFloat(String(value == null ? '' : value));
  if (!Number.isFinite(parsed)) return fallback;
  return Math.min(max, Math.max(min, parsed));
}

function asBool(value) {
  if (value === true) return true;
  const raw = String(value == null ? '' : value).trim().toLowerCase();
  if (!raw) return false;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function cleanString(value) {
  return String(value == null ? '' : value).trim();
}

function sessionIdFromKey(value) {
  const raw = cleanString(value);
  if (!raw) return '';
  if (!raw.includes(':')) return raw;
  const parts = raw.split(':').map((p) => p.trim()).filter(Boolean);
  if (parts.length === 0) return raw;
  return parts[parts.length - 1];
}

function asSessionKey(sessionId) {
  const id = cleanString(sessionId);
  if (!id) return '';
  return `agent:main:subagent:${id}`;
}

function execOps(args, env = {}) {
  const run = spawnSync(process.execPath, [OPS_WRAPPER].concat(args), {
    cwd: ROOT,
    encoding: 'utf8',
    env: { ...process.env, ...env },
  });
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  return {
    status,
    stdout: String(run.stdout || ''),
    stderr: String(run.stderr || ''),
    payload: parseLastJson(run.stdout),
  };
}

function printOpsOutput(result) {
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
}

function requireOk(result, label) {
  if (result.status !== 0) {
    throw new Error(`${label}_failed:status=${result.status}:${result.stderr || result.stdout}`);
  }
  if (!result.payload || result.payload.ok !== true) {
    throw new Error(`${label}_invalid_payload`);
  }
  return result.payload;
}

function normalizeSpawnPayload(payload) {
  const sessionId =
    (payload && payload.payload && payload.payload.session_id) ||
    (payload && payload.session_id) ||
    '';
  const toolAccess =
    (payload
      && payload.payload
      && payload.payload.session_state
      && payload.payload.session_state.tool_access)
    || [
      'sessions_spawn',
      'sessions_send',
      'sessions_receive',
      'sessions_ack',
      'sessions_handoff',
      'sessions_context_put',
      'sessions_context_get',
      'sessions_query',
      'sessions_state',
      'sessions_tick',
      'tools_register_json_schema',
      'tools_invoke',
      'stream_emit',
      'stream_render',
      'turns_run',
      'turns_show',
      'networks_create',
      'networks_status',
    ];
  const toolManifest =
    (payload
      && payload.payload
      && payload.payload.session_state
      && payload.payload.session_state.tool_manifest)
    || null;
  const agentBootstrap =
    (toolManifest && toolManifest.agent_bootstrap)
    || null;
  return {
    ok: true,
    type: 'sessions_spawn',
    session_id: sessionId,
    session_key: asSessionKey(sessionId),
    tool_access: Array.isArray(toolAccess) ? toolAccess : [],
    tool_manifest: toolManifest,
    agent_bootstrap: agentBootstrap,
    payload,
  };
}

function normalizeSendPayload(payload) {
  return {
    ok: true,
    type: 'sessions_send',
    message_id: payload.message_id || null,
    delivery: payload.delivery || null,
    attempts: payload.attempts || null,
    payload,
  };
}

function normalizeReceivePayload(payload, sessionId) {
  return {
    ok: true,
    type: 'sessions_receive',
    session_id: sessionId,
    session_key: asSessionKey(sessionId),
    message_count: payload.message_count || 0,
    messages: Array.isArray(payload.messages) ? payload.messages : [],
    payload,
  };
}

function normalizeStatePayload(payload, sessionId) {
  const agentBootstrap =
    payload
    && payload.session
    && payload.session.tool_manifest
    && payload.session.tool_manifest.agent_bootstrap;
  return {
    ok: true,
    type: 'sessions_state',
    session_id: sessionId,
    session_key: asSessionKey(sessionId),
    agent_bootstrap: agentBootstrap || null,
    payload,
  };
}

function normalizeBootstrapPayload(payload, sessionId) {
  return {
    ok: true,
    type: 'sessions_bootstrap',
    session_id: sessionId,
    session_key: asSessionKey(sessionId),
    bootstrap: payload.bootstrap || null,
    payload,
  };
}

function sessionsSpawn(options = {}) {
  const parsed = normalizedOptions(options);
  const args = ['swarm-runtime', 'spawn'];
  const task = cleanString(
    parsed.task || parsed.objective || parsed.prompt || parsed.message || 'swarm-session-task'
  );
  args.push(`--task=${task}`);

  const parent = sessionIdFromKey(
    parsed.session_id || parsed.sessionId || parsed.parent_session_id || parsed.parentSessionId
  );
  if (parent) args.push(`--session-id=${parent}`);

  if (asBool(parsed.recursive)) args.push('--recursive=1');
  if (parsed.levels != null) args.push(`--levels=${asInt(parsed.levels, 2, 1)}`);
  if (parsed.max_depth != null || parsed.maxDepth != null) {
    args.push(`--max-depth=${asInt(parsed.max_depth ?? parsed.maxDepth, 8, 1)}`);
  }

  const tokenBudget = cleanString(
    parsed.token_budget
      ?? parsed['token-budget']
      ?? parsed.max_tokens
      ?? parsed.maxTokens
      ?? parsed['max-tokens']
  );
  if (tokenBudget) args.push(`--token-budget=${asInt(tokenBudget, 1, 1)}`);
  const tokenWarningAt = cleanString(parsed.token_warning_at ?? parsed['token-warning-at']);
  if (tokenWarningAt) args.push(`--token-warning-at=${asFloat(tokenWarningAt, 0.8, 0, 1)}`);
  const budgetMode = cleanString(parsed.on_budget_exhausted ?? parsed['on-budget-exhausted']).toLowerCase();
  if (budgetMode === 'fail' || budgetMode === 'warn' || budgetMode === 'compact') {
    args.push(`--on-budget-exhausted=${budgetMode}`);
  } else if (tokenBudget) {
    // Fail-closed by default whenever a budget is explicitly requested.
    args.push('--on-budget-exhausted=fail');
  }
  if (parsed.adaptive_complexity != null || parsed['adaptive-complexity'] != null) {
    args.push(`--adaptive-complexity=${asBool(parsed.adaptive_complexity ?? parsed['adaptive-complexity']) ? 1 : 0}`);
  }

  const role = cleanString(parsed.agentRole || parsed.role);
  if (role) args.push(`--role=${role}`);
  const label = cleanString(parsed.agentLabel || parsed.agent_label || parsed.label);
  if (label) args.push(`--agent-label=${label}`);
  const capabilities = cleanString(parsed.capabilities);
  if (capabilities) args.push(`--capabilities=${capabilities}`);

  const sessionType = cleanString(parsed.sessionType || parsed.session_type).toLowerCase();
  if (sessionType === 'persistent' || sessionType === 'background') {
    args.push(`--execution-mode=${sessionType}`);
    const ttlMinutes = asInt(parsed.ttlMinutes ?? parsed.ttl_minutes, 60, 1);
    args.push(`--lifespan-sec=${ttlMinutes * 60}`);
    const checkpointSec = asInt(
      parsed.checkpointInterval ?? parsed.checkpoint_interval_sec,
      60,
      1
    );
    args.push(`--check-in-interval-sec=${checkpointSec}`);
  }

  const autoPublish = parsed.auto_publish_results ?? parsed.autoPublishResults;
  if (autoPublish != null) args.push(`--auto-publish-results=${asBool(autoPublish) ? 1 : 0}`);

  const testMode = cleanString(parsed.testMode || parsed.test_mode).toLowerCase();
  if (testMode === 'byzantine' || asBool(parsed.byzantine)) {
    const enable = execOps(['swarm-runtime', 'byzantine-test', 'enable', `--state-path=${statePath(parsed)}`]);
    requireOk(enable, 'byzantine_test_enable');
    args.push('--byzantine=1');
    let corruptionType = cleanString(parsed.corruption_type || parsed.corruptionType);
    if (!corruptionType && parsed.faultPattern) {
      try {
        const pattern =
          typeof parsed.faultPattern === 'string'
            ? JSON.parse(parsed.faultPattern)
            : parsed.faultPattern;
        corruptionType = cleanString(pattern.type || pattern.value);
      } catch {}
    }
    if (corruptionType) args.push(`--corruption-type=${corruptionType}`);
  }

  args.push(`--state-path=${statePath(parsed)}`);
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_spawn');
  return normalizeSpawnPayload(payload);
}

function sessionsSend(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const senderId = sessionIdFromKey(
    parsed.sender_session_key ||
      parsed.sender_session_id ||
      parsed.senderSessionKey ||
      parsed.senderSessionId ||
      parsed.sender ||
      'coordinator'
  );
  const message = cleanString(parsed.message || parsed.payload);
  const delivery = cleanString(parsed.delivery || 'at_least_once');
  const ttlMs = asInt(parsed.ttl_ms ?? parsed.ttlMs, 300000, 1);
  const args = [
    'swarm-runtime',
    'sessions',
    'send',
    `--sender-id=${senderId || 'coordinator'}`,
    `--session-id=${sessionId}`,
    `--message=${message}`,
    `--delivery=${delivery || 'at_least_once'}`,
    `--ttl-ms=${ttlMs}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_send');
  return normalizeSendPayload(payload);
}

function sessionsResume(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'resume',
    `--session-id=${sessionId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_resume');
  return {
    ok: true,
    type: 'sessions_resume',
    session_id: sessionId,
    payload,
  };
}

function sessionsBootstrap(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'bootstrap',
    `--session-id=${sessionId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_bootstrap');
  return normalizeBootstrapPayload(payload, sessionId);
}

function sessionsHandoff(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const targetSessionId = sessionIdFromKey(
    parsed.targetSessionKey || parsed.target_session_key || parsed.target_session_id || parsed.targetSessionId
  );
  const reason = cleanString(parsed.reason || parsed.message || 'handoff');
  const args = [
    'swarm-runtime',
    'sessions',
    'handoff',
    `--session-id=${sessionId}`,
    `--target-session-id=${targetSessionId}`,
    `--reason=${reason}`,
    `--importance=${asFloat(parsed.importance, 0.5, 0, 1)}`,
    `--state-path=${statePath(parsed)}`,
  ];
  if (parsed.context_json || parsed.context) {
    const context = typeof parsed.context_json === 'string'
      ? parsed.context_json
      : JSON.stringify(parsed.context_json || parsed.context);
    args.push(`--context-json=${context}`);
  }
  if (parsed.network_id || parsed.networkId) {
    args.push(`--network-id=${cleanString(parsed.network_id || parsed.networkId)}`);
  }
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_handoff');
  return {
    ok: true,
    type: 'sessions_handoff',
    session_id: sessionId,
    target_session_id: targetSessionId,
    payload,
  };
}

function sessionsContextPut(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const context = parsed.context_json || parsed.context || {};
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'context-put',
    `--session-id=${sessionId}`,
    `--context-json=${typeof context === 'string' ? context : JSON.stringify(context)}`,
    `--merge=${asBool(parsed.merge != null ? parsed.merge : true) ? 1 : 0}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_context_put');
  return {
    ok: true,
    type: 'sessions_context_put',
    session_id: sessionId,
    payload,
  };
}

function sessionsContextGet(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'context-get',
    `--session-id=${sessionId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_context_get');
  return {
    ok: true,
    type: 'sessions_context_get',
    session_id: sessionId,
    context: payload.context || {},
    payload,
  };
}

function toolsRegisterJsonSchema(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const toolName = cleanString(parsed.toolName || parsed.tool_name);
  const schema = parsed.schema_json || parsed.schema || {};
  const bridgePath = cleanString(parsed.bridgePath || parsed.bridge_path);
  const entrypoint = cleanString(parsed.entrypoint || parsed.command || toolName);
  const args = [
    'swarm-runtime',
    'tools',
    'register-json-schema',
    `--session-id=${sessionId}`,
    `--tool-name=${toolName}`,
    `--schema-json=${typeof schema === 'string' ? schema : JSON.stringify(schema)}`,
    `--bridge-path=${bridgePath}`,
    `--entrypoint=${entrypoint}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const description = cleanString(parsed.description);
  if (description) args.push(`--description=${description}`);
  const run = execOps(args);
  const payload = requireOk(run, 'tools_register_json_schema');
  return {
    ok: true,
    type: 'tools_register_json_schema',
    session_id: sessionId,
    tool_name: toolName,
    payload,
  };
}

function toolsInvoke(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const toolName = cleanString(parsed.toolName || parsed.tool_name);
  const argsJson = parsed.args_json || parsed.args || {};
  const run = execOps([
    'swarm-runtime',
    'tools',
    'invoke',
    `--session-id=${sessionId}`,
    `--tool-name=${toolName}`,
    `--args-json=${typeof argsJson === 'string' ? argsJson : JSON.stringify(argsJson)}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'tools_invoke');
  return {
    ok: true,
    type: 'tools_invoke',
    session_id: sessionId,
    tool_name: toolName,
    payload,
  };
}

function streamEmit(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const chunks = parsed.chunks_json || parsed.chunks || [];
  const args = [
    'swarm-runtime',
    'stream',
    'emit',
    `--session-id=${sessionId}`,
    `--chunks-json=${typeof chunks === 'string' ? chunks : JSON.stringify(chunks)}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const turnId = cleanString(parsed.turnId || parsed.turn_id);
  if (turnId) args.push(`--turn-id=${turnId}`);
  const agentLabel = cleanString(parsed.agentLabel || parsed.agent_label);
  if (agentLabel) args.push(`--agent-label=${agentLabel}`);
  const run = execOps(args);
  const payload = requireOk(run, 'stream_emit');
  return {
    ok: true,
    type: 'stream_emit',
    session_id: sessionId,
    payload,
  };
}

function streamRender(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const args = [
    'swarm-runtime',
    'stream',
    'render',
    `--session-id=${sessionId}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const turnId = cleanString(parsed.turnId || parsed.turn_id);
  if (turnId) args.push(`--turn-id=${turnId}`);
  const run = execOps(args);
  const payload = requireOk(run, 'stream_render');
  return {
    ok: true,
    type: 'stream_render',
    session_id: sessionId,
    payload,
  };
}

function turnsRun(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const turns = parsed.turns_json || parsed.turns || [];
  const args = [
    'swarm-runtime',
    'turns',
    'run',
    `--session-id=${sessionId}`,
    `--turns-json=${typeof turns === 'string' ? turns : JSON.stringify(turns)}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const label = cleanString(parsed.label);
  if (label) args.push(`--label=${label}`);
  const run = execOps(args);
  const payload = requireOk(run, 'turns_run');
  return {
    ok: true,
    type: 'turns_run',
    session_id: sessionId,
    payload,
  };
}

function turnsShow(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const runId = cleanString(parsed.runId || parsed.run_id);
  const run = execOps([
    'swarm-runtime',
    'turns',
    'show',
    `--session-id=${sessionId}`,
    `--run-id=${runId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'turns_show');
  return {
    ok: true,
    type: 'turns_show',
    session_id: sessionId,
    run_id: runId,
    payload,
  };
}

function networksCreate(options = {}) {
  const parsed = normalizedOptions(options);
  const args = ['swarm-runtime', 'networks', 'create', `--state-path=${statePath(parsed)}`];
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  if (sessionId) args.push(`--session-id=${sessionId}`);
  const spec = parsed.spec_json || parsed.spec || {};
  args.push(`--spec-json=${typeof spec === 'string' ? spec : JSON.stringify(spec)}`);
  const run = execOps(args);
  const payload = requireOk(run, 'networks_create');
  return {
    ok: true,
    type: 'networks_create',
    payload,
  };
}

function networksStatus(options = {}) {
  const parsed = normalizedOptions(options);
  const args = [
    'swarm-runtime',
    'networks',
    'status',
    `--network-id=${cleanString(parsed.networkId || parsed.network_id)}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  if (sessionId) args.push(`--session-id=${sessionId}`);
  const run = execOps(args);
  const payload = requireOk(run, 'networks_status');
  return {
    ok: true,
    type: 'networks_status',
    payload,
  };
}

function sessionsDeadLetters(options = {}) {
  const parsed = normalizedOptions(options);
  const args = ['swarm-runtime', 'sessions', 'dead-letter'];
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  if (sessionId) args.push(`--session-id=${sessionId}`);
  if (parsed.retryable != null) args.push(`--retryable=${asBool(parsed.retryable) ? 1 : 0}`);
  args.push(`--state-path=${statePath(parsed)}`);
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_dead_letter');
  return {
    ok: true,
    type: 'sessions_dead_letter',
    payload,
  };
}

function sessionsRetryDeadLetter(options = {}) {
  const parsed = normalizedOptions(options);
  const messageId = cleanString(parsed.message_id || parsed.messageId);
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'retry-dead-letter',
    `--message-id=${messageId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_retry_dead_letter');
  return {
    ok: true,
    type: 'sessions_retry_dead_letter',
    message_id: messageId,
    payload,
  };
}

function sessionsReceive(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const limit = asInt(parsed.limit, 10, 1);
  const args = [
    'swarm-runtime',
    'sessions',
    'receive',
    `--session-id=${sessionId}`,
    `--limit=${limit}`,
    '--mark-read=0',
    `--state-path=${statePath(parsed)}`,
  ];
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_receive');
  return normalizeReceivePayload(payload, sessionId);
}

function sessionsAck(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const messageId = cleanString(parsed.message_id || parsed.messageId);
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'ack',
    `--session-id=${sessionId}`,
    `--message-id=${messageId}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_ack');
  return {
    ok: true,
    type: 'sessions_ack',
    session_id: sessionId,
    message_id: messageId,
    payload,
  };
}

function sessionsState(options = {}) {
  const parsed = normalizedOptions(options);
  const sessionId = sessionIdFromKey(
    parsed.sessionKey || parsed.session_key || parsed.session_id || parsed.sessionId
  );
  const timeline = asBool(parsed.timeline) ? 1 : 0;
  const toolHistoryLimit = asInt(parsed.tool_history_limit ?? parsed.toolHistoryLimit, 32, 1);
  const run = execOps([
    'swarm-runtime',
    'sessions',
    'state',
    `--session-id=${sessionId}`,
    `--timeline=${timeline}`,
    `--tool-history-limit=${toolHistoryLimit}`,
    `--state-path=${statePath(parsed)}`,
  ]);
  const payload = requireOk(run, 'sessions_state');
  return normalizeStatePayload(payload, sessionId);
}

function sessionsQuery(options = {}) {
  const parsed = normalizedOptions(options);
  const role = cleanString(parsed.agentRole || parsed.role);
  const label = cleanString(parsed.agentLabel || parsed.agent_label || parsed.label);
  const taskId = cleanString(parsed.testId || parsed.task_id || parsed.taskId);
  const sessionId = sessionIdFromKey(parsed.session_id || parsed.sessionId);
  const wait = asBool(parsed.wait);
  const args = ['swarm-runtime', 'results', wait ? 'wait' : 'query'];
  if (role) args.push(`--role=${role}`);
  if (label) args.push(`--label-pattern=${label}`);
  if (taskId) args.push(`--task-id=${taskId}`);
  if (sessionId) args.push(`--session-id=${sessionId}`);
  if (wait) {
    args.push(`--min-count=${asInt(parsed.min_count ?? parsed.minCount, 1, 1)}`);
    args.push(`--timeout-sec=${asInt(parsed.timeout_sec ?? parsed.timeoutSec, 10, 1)}`);
  }
  args.push(`--state-path=${statePath(parsed)}`);
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_query');

  let discovery = null;
  if (role) {
    const discoverRun = execOps([
      'swarm-runtime',
      'sessions',
      'discover',
      `--role=${role}`,
      `--state-path=${statePath(parsed)}`,
    ]);
    if (discoverRun.status === 0) {
      discovery = discoverRun.payload || null;
    }
  }

  return {
    ok: true,
    type: 'sessions_query',
    result_count: payload.result_count || 0,
    results: Array.isArray(payload.results) ? payload.results : [],
    discovery,
    payload,
  };
}

function sessionsTick(options = {}) {
  const parsed = normalizedOptions(options);
  const args = [
    'swarm-runtime',
    'tick',
    `--advance-ms=${asInt(parsed.advance_ms ?? parsed.advanceMs, 1000, 1)}`,
    `--max-check-ins=${asInt(parsed.max_check_ins ?? parsed.maxCheckIns, 32, 1)}`,
    `--state-path=${statePath(parsed)}`,
  ];
  const run = execOps(args);
  const payload = requireOk(run, 'sessions_tick');
  return {
    ok: true,
    type: 'sessions_tick',
    payload,
  };
}

function printUsage() {
  process.stdout.write(
    [
      'Usage:',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_spawn --task=<text> [--session-id=<parent>] [--sessionType=persistent|background] [--ttlMinutes=<n>] [--checkpointInterval=<sec>] [--token-budget=<n>|--max-tokens=<n>] [--testMode=byzantine] [--faultPattern=\'{\"type\":\"corruption\"}\'] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_send --sessionKey=<key|id> --message=<text> [--sender=<key|id>] [--delivery=<at_most_once|at_least_once|exactly_once>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_receive --sessionKey=<key|id> [--limit=<n>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_ack --sessionKey=<key|id> --message-id=<id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_handoff --sessionKey=<key|id> --targetSessionKey=<key|id> --reason=<text> [--importance=<0..1>] [--context-json=<json>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_context_put --sessionKey=<key|id> --context-json=<json> [--merge=1|0] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_context_get --sessionKey=<key|id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_resume --sessionKey=<key|id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_bootstrap --sessionKey=<key|id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_dead_letter [--sessionKey=<key|id>] [--retryable=1|0] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_retry_dead_letter --message-id=<id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_query [--agentRole=<role>] [--agentLabel=<label>] [--testId=<task>] [--wait=1] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_state --sessionKey=<key|id> [--timeline=1] [--tool-history-limit=<n>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts sessions_tick [--advance-ms=<n>] [--max-check-ins=<n>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts tools_register_json_schema --sessionKey=<key|id> --toolName=<name> --schema-json=<json> --bridgePath=<path> --entrypoint=<name> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts tools_invoke --sessionKey=<key|id> --toolName=<name> --args-json=<json> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts stream_emit --sessionKey=<key|id> --chunks-json=<json> [--turn-id=<id>] [--agentLabel=<label>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts stream_render --sessionKey=<key|id> [--turn-id=<id>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts turns_run --sessionKey=<key|id> --turns-json=<json> [--label=<text>] [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts turns_show --sessionKey=<key|id> --runId=<id> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts networks_create [--sessionKey=<key|id>] --spec-json=<json> [--state-path=<path>]',
      '  node client/runtime/systems/autonomy/swarm_sessions_bridge.ts networks_status --networkId=<id> [--sessionKey=<key|id>] [--state-path=<path>]',
      '',
      'Aliases: spawn/send/receive/ack/handoff/context-put/context-get/resume/bootstrap/query/state/tick',
      '',
    ].join('\n')
  );
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = cleanString(parsed._[0] || 'sessions_spawn').toLowerCase();

  let payload;
  if (command === 'help' || command === '--help' || command === '-h') {
    printUsage();
    return 0;
  }
  if (command === 'sessions_spawn' || command === 'spawn') payload = sessionsSpawn(parsed);
  else if (command === 'sessions_send' || command === 'send') payload = sessionsSend(parsed);
  else if (command === 'sessions_receive' || command === 'receive') payload = sessionsReceive(parsed);
  else if (command === 'sessions_ack' || command === 'ack') payload = sessionsAck(parsed);
  else if (command === 'sessions_handoff' || command === 'handoff') payload = sessionsHandoff(parsed);
  else if (command === 'sessions_context_put' || command === 'context-put') payload = sessionsContextPut(parsed);
  else if (command === 'sessions_context_get' || command === 'context-get') payload = sessionsContextGet(parsed);
  else if (command === 'sessions_resume' || command === 'resume') payload = sessionsResume(parsed);
  else if (command === 'sessions_bootstrap' || command === 'bootstrap') payload = sessionsBootstrap(parsed);
  else if (command === 'sessions_dead_letter' || command === 'dead-letter') payload = sessionsDeadLetters(parsed);
  else if (command === 'sessions_retry_dead_letter' || command === 'retry-dead-letter') payload = sessionsRetryDeadLetter(parsed);
  else if (command === 'sessions_query' || command === 'query') payload = sessionsQuery(parsed);
  else if (command === 'sessions_state' || command === 'state') payload = sessionsState(parsed);
  else if (command === 'sessions_tick' || command === 'tick') payload = sessionsTick(parsed);
  else if (command === 'tools_register_json_schema' || command === 'register-json-schema') payload = toolsRegisterJsonSchema(parsed);
  else if (command === 'tools_invoke' || command === 'tool-invoke') payload = toolsInvoke(parsed);
  else if (command === 'stream_emit' || command === 'stream-emit') payload = streamEmit(parsed);
  else if (command === 'stream_render' || command === 'stream-render') payload = streamRender(parsed);
  else if (command === 'turns_run' || command === 'turns-run') payload = turnsRun(parsed);
  else if (command === 'turns_show' || command === 'turns-show') payload = turnsShow(parsed);
  else if (command === 'networks_create' || command === 'networks-create') payload = networksCreate(parsed);
  else if (command === 'networks_status' || command === 'networks-status') payload = networksStatus(parsed);
  else {
    process.stderr.write(`unknown_command:${command}\n`);
    printUsage();
    return 2;
  }

  process.stdout.write(`${JSON.stringify(payload)}\n`);
  return 0;
}

if (require.main === module) {
  try {
    process.exit(run(process.argv.slice(2)));
  } catch (err) {
    process.stderr.write(`${String((err && err.message) || err)}\n`);
    process.exit(1);
  }
}

module.exports = {
  ROOT,
  DEFAULT_STATE_PATH,
  parseArgs,
  sessionsSpawn,
  sessionsSend,
  sessionsReceive,
  sessionsAck,
  sessionsHandoff,
  sessionsContextPut,
  sessionsContextGet,
  sessionsResume,
  sessionsBootstrap,
  sessionsDeadLetters,
  sessionsRetryDeadLetter,
  sessionsQuery,
  sessionsState,
  sessionsTick,
  toolsRegisterJsonSchema,
  toolsInvoke,
  streamEmit,
  streamRender,
  turnsRun,
  turnsShow,
  networksCreate,
  networksStatus,
  run,
};
