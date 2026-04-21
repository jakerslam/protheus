#!/usr/bin/env node
'use strict';

const path = require('node:path');
const {
  ROOT,
  parseArgs,
  parseJson,
  invokeOrchestration,
} = require('./core_bridge.ts');
const { runTaskGroupCli } = require('./taskgroup_cli.ts');

const DEFAULT_TASKGROUP_DIR = path.join(ROOT, 'local', 'workspace', 'scratchpad', 'taskgroups');
const TASKGROUP_SCHEMA_VERSION = 'taskgroup/v1';
const GROUP_ID_PATTERN = /^[a-z0-9][a-z0-9._:-]{5,127}$/;
const AGENT_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{1,127}$/;
const ALLOWED_AGENT_STATUSES = new Set(['pending', 'running', 'done', 'failed', 'timeout']);
const TERMINAL_AGENT_STATUSES = new Set(['done', 'failed', 'timeout']);

function normalizeTaskGroupResponse(out, fallbackType) {
  if (!out || typeof out !== 'object') {
    return {
      ok: false,
      type: fallbackType,
      reason_code: 'orchestration_bridge_error',
    };
  }

  const response = {
    ok: Boolean(out.ok),
    type: String(out.type || fallbackType),
    reason_code: out.reason_code ? String(out.reason_code) : undefined,
  };

  if (typeof out.created === 'boolean') response.created = out.created;
  if (out.file_path) {
    response.file_path = String(out.file_path);
    response.filePath = String(out.file_path);
  }
  if (out.task_group && typeof out.task_group === 'object') response.task_group = out.task_group;
  if (out.counts && typeof out.counts === 'object') response.counts = out.counts;
  if (out.task_group_id) response.task_group_id = String(out.task_group_id);
  if (out.agent_id) response.agent_id = String(out.agent_id);
  if (out.status) response.status = String(out.status);
  if (out.previous_status) response.previous_status = String(out.previous_status);

  return response;
}

function generateTaskGroupId(taskType = 'task', options = {}) {
  const out = invokeOrchestration('taskgroup.generate_id', {
    task_type: String(taskType || '').trim() || 'task',
    now_ms: Number.isFinite(Number(options.now_ms)) ? Number(options.now_ms) : Date.now(),
    nonce: String(options.nonce || '').trim(),
  });
  return out && out.ok && out.task_group_id ? String(out.task_group_id).trim().toLowerCase() : '';
}

function taskGroupPath(taskGroupId, options = {}) {
  const out = invokeOrchestration('taskgroup.path', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  if (out && out.ok && out.file_path) return String(out.file_path);
  throw new Error(String(out && out.reason_code ? out.reason_code : 'orchestration_bridge_error'));
}

function statusCounts(group) {
  const out = invokeOrchestration('taskgroup.status_counts', {
    task_group: group && typeof group === 'object' ? group : {},
  });
  if (out && out.ok && out.counts && typeof out.counts === 'object') {
    return out.counts;
  }
  return {
    pending: 0,
    running: 0,
    done: 0,
    failed: 0,
    timeout: 0,
    total: 0,
  };
}

function deriveGroupStatus(group) {
  const out = invokeOrchestration('taskgroup.derive_status', {
    task_group: group && typeof group === 'object' ? group : {},
  });
  return out && out.ok && out.status ? String(out.status).trim().toLowerCase() : 'pending';
}

function ensureTaskGroup(input = {}, options = {}) {
  const out = invokeOrchestration('taskgroup.ensure', {
    ...(input && typeof input === 'object' ? input : {}),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  return normalizeTaskGroupResponse(out, 'orchestration_taskgroup_ensure');
}

function updateAgentStatus(taskGroupId, agentId, status, details = {}, options = {}) {
  const out = invokeOrchestration('taskgroup.update_status', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    agent_id: String(agentId || '').trim(),
    status: String(status || '').trim().toLowerCase(),
    details: details && typeof details === 'object' && !Array.isArray(details) ? details : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  return normalizeTaskGroupResponse(out, 'orchestration_taskgroup_update_status');
}

function queryTaskGroup(taskGroupId, options = {}) {
  const out = invokeOrchestration('taskgroup.query', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  return normalizeTaskGroupResponse(out, 'orchestration_taskgroup_query');
}

function loadTaskGroup(taskGroupId, options = {}) {
  const query = queryTaskGroup(taskGroupId, options);
  if (query.ok) {
    return {
      ok: true,
      exists: true,
      file_path: query.file_path,
      filePath: query.file_path,
      task_group: query.task_group,
    };
  }

  return {
    ok: true,
    exists: false,
    file_path: taskGroupPath(taskGroupId, options),
    filePath: taskGroupPath(taskGroupId, options),
    task_group: null,
  };
}

function saveTaskGroup(taskGroup, options = {}) {
  if (!taskGroup || typeof taskGroup !== 'object' || Array.isArray(taskGroup)) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: 'invalid_taskgroup',
    };
  }

  const ensured = ensureTaskGroup(taskGroup, options);
  if (!ensured.ok) return ensured;

  const taskGroupId = ensured.task_group && ensured.task_group.task_group_id
    ? ensured.task_group.task_group_id
    : taskGroup.task_group_id;

  const updates = Array.isArray(taskGroup.agents) ? taskGroup.agents : [];
  for (const agent of updates) {
    const status = String(agent && agent.status ? agent.status : 'pending').toLowerCase();
    if (!ALLOWED_AGENT_STATUSES.has(status)) continue;
    const out = updateAgentStatus(
      taskGroupId,
      String(agent && agent.agent_id ? agent.agent_id : ''),
      status,
      agent && typeof agent.details === 'object' ? agent.details : {},
      options,
    );
    if (!out.ok) return out;
  }

  const queried = queryTaskGroup(taskGroupId, options);
  if (!queried.ok) return queried;

  return {
    ok: true,
    type: 'orchestration_taskgroup_save',
    file_path: queried.file_path,
    filePath: queried.file_path,
    task_group: queried.task_group,
    counts: queried.counts,
  };
}

function run(argv = process.argv.slice(2)) {
  return runTaskGroupCli(argv, {
    ensureTaskGroup,
    queryTaskGroup,
    updateAgentStatus,
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
  AGENT_ID_PATTERN,
  ALLOWED_AGENT_STATUSES,
  TERMINAL_AGENT_STATUSES,
  parseArgs,
  parseJson,
  generateTaskGroupId,
  taskGroupPath,
  statusCounts,
  deriveGroupStatus,
  loadTaskGroup,
  saveTaskGroup,
  ensureTaskGroup,
  updateAgentStatus,
  queryTaskGroup,
  run,
};
