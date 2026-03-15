#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const {
  parseArgs,
  slug,
  timestampToken,
  nonceToken
} = require('./cli_shared.ts');
const { runTaskGroupCli } = require('./taskgroup_cli.ts');

const ROOT = path.resolve(__dirname, '..', '..', '..');
const DEFAULT_TASKGROUP_DIR = path.join(ROOT, 'local', 'workspace', 'scratchpad', 'taskgroups');
const TASKGROUP_SCHEMA_VERSION = 'taskgroup/v1';
const GROUP_ID_PATTERN = /^[a-z0-9][a-z0-9._:-]{5,127}$/;
const AGENT_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{1,127}$/;
const ALLOWED_AGENT_STATUSES = new Set(['pending', 'running', 'done', 'failed', 'timeout']);
const TERMINAL_AGENT_STATUSES = new Set(['done', 'failed', 'timeout']);

function generateTaskGroupId(taskType = 'task', options = {}) {
  const nowMs = Number.isFinite(Number(options.now_ms)) ? Number(options.now_ms) : Date.now();
  const nonce = String(options.nonce || '').trim().toLowerCase() || nonceToken(6);
  const id = `${slug(taskType, 'task')}-${timestampToken(nowMs)}-${slug(nonce, nonceToken(6))}`;
  return id.slice(0, 127);
}

function taskGroupPath(taskGroupId, options = {}) {
  const id = String(taskGroupId || '').trim().toLowerCase();
  if (!GROUP_ID_PATTERN.test(id)) {
    throw new Error(`invalid_task_group_id:${taskGroupId || '<empty>'}`);
  }
  const rootDir = options.rootDir || DEFAULT_TASKGROUP_DIR;
  return path.join(rootDir, `${id}.json`);
}

function normalizeAgentId(raw, index = 0) {
  const id = String(raw || '').trim() || `agent-${index + 1}`;
  if (!AGENT_ID_PATTERN.test(id)) {
    throw new Error(`invalid_agent_id:${id}`);
  }
  return id;
}

function normalizeAgents(inputAgents = [], fallbackCount = 1) {
  const source = Array.isArray(inputAgents) ? inputAgents : [];
  const out = [];
  const seen = new Set();

  for (let index = 0; index < source.length; index += 1) {
    const row = source[index];
    const agentId = normalizeAgentId(
      row && typeof row === 'object' && !Array.isArray(row) ? row.agent_id || row.agentId : row,
      index
    );
    if (seen.has(agentId)) continue;
    seen.add(agentId);
    const statusRaw = String(
      row && typeof row === 'object' && !Array.isArray(row) ? row.status || 'pending' : 'pending'
    ).toLowerCase();
    const status = ALLOWED_AGENT_STATUSES.has(statusRaw) ? statusRaw : 'pending';
    out.push({
      agent_id: agentId,
      status,
      updated_at: new Date().toISOString(),
      details: row && typeof row === 'object' && row.details && typeof row.details === 'object'
        ? row.details
        : {}
    });
  }

  const desiredCount = Math.max(1, Number.isFinite(Number(fallbackCount)) ? Number(fallbackCount) : 1);
  while (out.length < desiredCount) {
    const nextId = normalizeAgentId(`agent-${out.length + 1}`, out.length);
    if (seen.has(nextId)) continue;
    seen.add(nextId);
    out.push({
      agent_id: nextId,
      status: 'pending',
      updated_at: new Date().toISOString(),
      details: {}
    });
  }

  return out;
}

function statusCounts(group) {
  const counts = {
    pending: 0,
    running: 0,
    done: 0,
    failed: 0,
    timeout: 0,
    total: 0
  };
  const agents = Array.isArray(group && group.agents) ? group.agents : [];
  for (const agent of agents) {
    const status = String(agent && agent.status ? agent.status : 'pending').toLowerCase();
    if (!(status in counts)) continue;
    counts[status] += 1;
    counts.total += 1;
  }
  return counts;
}

function deriveGroupStatus(group) {
  const counts = statusCounts(group);
  if (counts.total === 0 || counts.pending === counts.total) return 'pending';
  if (counts.running > 0 || counts.pending > 0) return 'running';
  if (counts.failed > 0 && counts.done === 0 && counts.timeout === 0) return 'failed';
  if (counts.timeout > 0 && counts.done === 0 && counts.failed === 0) return 'timeout';
  if (counts.done === counts.total) return 'done';
  if (counts.done + counts.failed + counts.timeout === counts.total) return 'completed';
  return 'running';
}

function defaultTaskGroup(taskGroupId, input = {}) {
  const now = new Date().toISOString();
  const agentCount = Math.max(1, Number.isFinite(Number(input.agent_count)) ? Number(input.agent_count) : 1);
  const agents = normalizeAgents(input.agents, agentCount);
  return {
    schema_version: TASKGROUP_SCHEMA_VERSION,
    task_group_id: taskGroupId,
    task_type: slug(input.task_type || 'task', 'task'),
    coordinator_session: String(input.coordinator_session || '').trim() || null,
    created_at: now,
    updated_at: now,
    agent_count: agents.length,
    status: 'pending',
    agents,
    history: []
  };
}

function loadTaskGroup(taskGroupId, options = {}) {
  const filePath = taskGroupPath(taskGroupId, options);
  try {
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    if (!parsed || typeof parsed !== 'object' || Array.isArray(parsed)) {
      throw new Error('invalid_taskgroup_payload');
    }
    parsed.schema_version = TASKGROUP_SCHEMA_VERSION;
    parsed.task_group_id = String(parsed.task_group_id || taskGroupId).trim().toLowerCase();
    parsed.agents = normalizeAgents(parsed.agents, parsed.agent_count || 1);
    parsed.agent_count = parsed.agents.length;
    parsed.history = Array.isArray(parsed.history) ? parsed.history : [];
    parsed.status = deriveGroupStatus(parsed);
    return {
      ok: true,
      exists: true,
      file_path: filePath,
      task_group: parsed
    };
  } catch {
    return {
      ok: true,
      exists: false,
      file_path: filePath,
      task_group: null
    };
  }
}

function saveTaskGroup(taskGroup, options = {}) {
  const group = taskGroup && typeof taskGroup === 'object' && !Array.isArray(taskGroup)
    ? Object.assign({}, taskGroup)
    : null;
  if (!group) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: 'invalid_taskgroup'
    };
  }

  const taskGroupId = String(group.task_group_id || '').trim().toLowerCase();
  let filePath;
  try {
    filePath = taskGroupPath(taskGroupId, options);
  } catch (error) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: String(error && error.message ? error.message : error)
    };
  }

  group.schema_version = TASKGROUP_SCHEMA_VERSION;
  group.agents = normalizeAgents(group.agents, group.agent_count || 1);
  group.agent_count = group.agents.length;
  group.status = deriveGroupStatus(group);
  group.updated_at = new Date().toISOString();
  if (!group.created_at) group.created_at = group.updated_at;
  if (!Array.isArray(group.history)) group.history = [];

  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(group, null, 2)}\n`);

  return {
    ok: true,
    type: 'orchestration_taskgroup_save',
    file_path: filePath,
    task_group: group,
    counts: statusCounts(group)
  };
}

function ensureTaskGroup(input = {}, options = {}) {
  const hasRequestedId = String(input.task_group_id || input.taskGroupId || '').trim();
  const taskGroupId = hasRequestedId
    ? String(input.task_group_id || input.taskGroupId).trim().toLowerCase()
    : generateTaskGroupId(input.task_type || input.taskType || 'task', {
      now_ms: input.now_ms,
      nonce: input.nonce
    });

  const loaded = loadTaskGroup(taskGroupId, options);
  if (!loaded.ok) return loaded;
  if (loaded.exists) {
    return {
      ok: true,
      type: 'orchestration_taskgroup_ensure',
      created: false,
      file_path: loaded.file_path,
      task_group: loaded.task_group,
      counts: statusCounts(loaded.task_group)
    };
  }

  const created = defaultTaskGroup(taskGroupId, {
    task_type: input.task_type || input.taskType || 'task',
    coordinator_session: input.coordinator_session || input.coordinatorSession || '',
    agent_count: input.agent_count || input.agentCount || 1,
    agents: input.agents
  });
  const saved = saveTaskGroup(created, options);
  if (!saved.ok) return saved;

  return {
    ok: true,
    type: 'orchestration_taskgroup_ensure',
    created: true,
    file_path: saved.file_path,
    task_group: saved.task_group,
    counts: saved.counts
  };
}

function updateAgentStatus(taskGroupId, agentId, status, details = {}, options = {}) {
  const ensure = ensureTaskGroup({ task_group_id: taskGroupId }, options);
  if (!ensure.ok) return ensure;

  const normalizedAgentId = normalizeAgentId(agentId || 'agent-1');
  const normalizedStatus = String(status || '').trim().toLowerCase();
  if (!ALLOWED_AGENT_STATUSES.has(normalizedStatus)) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_update_status',
      reason_code: `invalid_agent_status:${status || '<empty>'}`
    };
  }

  const group = ensure.task_group;
  const now = new Date().toISOString();

  let target = group.agents.find((row) => row.agent_id === normalizedAgentId);
  if (!target) {
    target = {
      agent_id: normalizedAgentId,
      status: 'pending',
      updated_at: now,
      details: {}
    };
    group.agents.push(target);
    group.agent_count = group.agents.length;
  }

  const previousStatus = target.status;
  target.status = normalizedStatus;
  target.updated_at = now;
  target.details = details && typeof details === 'object' && !Array.isArray(details)
    ? Object.assign({}, target.details || {}, details)
    : Object.assign({}, target.details || {});

  group.history = Array.isArray(group.history) ? group.history : [];
  group.history.push({
    event: 'agent_status_update',
    at: now,
    agent_id: normalizedAgentId,
    previous_status: previousStatus,
    status: normalizedStatus,
    terminal: TERMINAL_AGENT_STATUSES.has(normalizedStatus),
    details: target.details
  });

  const saved = saveTaskGroup(group, options);
  if (!saved.ok) return saved;

  return {
    ok: true,
    type: 'orchestration_taskgroup_update_status',
    task_group_id: saved.task_group.task_group_id,
    agent_id: normalizedAgentId,
    status: normalizedStatus,
    previous_status: previousStatus,
    file_path: saved.file_path,
    task_group: saved.task_group,
    counts: saved.counts
  };
}

function queryTaskGroup(taskGroupId, options = {}) {
  const loaded = loadTaskGroup(taskGroupId, options);
  if (!loaded.ok) return loaded;
  if (!loaded.exists) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_query',
      reason_code: 'task_group_not_found',
      task_group_id: String(taskGroupId || '').trim().toLowerCase()
    };
  }

  return {
    ok: true,
    type: 'orchestration_taskgroup_query',
    file_path: loaded.file_path,
    task_group: loaded.task_group,
    counts: statusCounts(loaded.task_group)
  };
}

function run(argv = process.argv.slice(2)) {
  return runTaskGroupCli(argv, {
    ensureTaskGroup,
    queryTaskGroup,
    updateAgentStatus
  });
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  ROOT,
  DEFAULT_TASKGROUP_DIR,
  TASKGROUP_SCHEMA_VERSION,
  GROUP_ID_PATTERN,
  ALLOWED_AGENT_STATUSES,
  TERMINAL_AGENT_STATUSES,
  parseArgs,
  generateTaskGroupId,
  taskGroupPath,
  statusCounts,
  deriveGroupStatus,
  loadTaskGroup,
  saveTaskGroup,
  ensureTaskGroup,
  updateAgentStatus,
  queryTaskGroup,
  run
};
