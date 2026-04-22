#!/usr/bin/env node
'use strict';

const path = require('node:path');
const {
  ROOT,
  parseArgs,
  parseJson,
  invokeOrchestration,
} = require('./core_bridge.ts');
const { shouldFallbackForUnsupportedOp } = require('./cli_shared.ts');
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

function listTaskGroupAgents(taskGroupId, options = {}) {
  const out = invokeOrchestration('taskgroup.list_agents', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  if (out && typeof out === 'object') {
    const normalized = {
      ok: Boolean(out.ok),
      type: String(out.type || 'orchestration_taskgroup_list_agents'),
      reason_code: out.reason_code ? String(out.reason_code) : undefined,
      task_group_id: out.task_group_id ? String(out.task_group_id) : String(taskGroupId || '').trim().toLowerCase(),
      agents: Array.isArray(out.agents) ? out.agents : [],
      counts: out.counts && typeof out.counts === 'object' ? out.counts : null,
    };
    if (!shouldFallbackForUnsupportedOp(normalized, 'taskgroup.list_agents')) {
      return normalized;
    }

    const query = queryTaskGroup(taskGroupId, options);
    if (query.ok) {
      return {
        ok: true,
        type: 'orchestration_taskgroup_list_agents',
        task_group_id: query.task_group_id || String(taskGroupId || '').trim().toLowerCase(),
        agents: query.task_group && Array.isArray(query.task_group.agents) ? query.task_group.agents : [],
        counts: query.counts || null,
      };
    }
    return {
      ok: false,
      type: normalized.type,
      reason_code: query.reason_code || normalized.reason_code,
      task_group_id: normalized.task_group_id,
      agents: [],
      counts: null,
    };
  }
  return {
    ok: false,
    type: 'orchestration_taskgroup_list_agents',
    reason_code: 'orchestration_bridge_error',
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    agents: [],
    counts: null,
  };
}

function normalizeAgentCompletionUpdate(agent) {
  const agentId = String(agent && agent.agent_id ? agent.agent_id : '').trim();
  if (!agentId) return null;

  const status = String(agent && agent.status ? agent.status : 'pending').trim().toLowerCase();
  if (!ALLOWED_AGENT_STATUSES.has(status)) return null;

  const details = agent && typeof agent.details === 'object' && !Array.isArray(agent.details)
    ? agent.details
    : {};

  return {
    agent_id: agentId,
    status,
    details,
  };
}

function batchUpdateAgentStatuses(taskGroupId, agents = [], options = {}) {
  const updates = Array.isArray(agents)
    ? agents
      .map((agent) => normalizeAgentCompletionUpdate(agent))
      .filter(Boolean)
    : [];
  if (!updates.length) {
    return {
      ok: true,
      type: 'orchestration_completion_track_batch',
      updates_applied: 0,
    };
  }

  const out = invokeOrchestration('completion.batch', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    updates,
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  if (out && typeof out === 'object') {
    return {
      ok: Boolean(out.ok),
      type: String(out.type || 'orchestration_completion_track_batch'),
      reason_code: out.reason_code ? String(out.reason_code) : undefined,
      updates_applied: Number.isFinite(Number(out.updates_applied)) ? Number(out.updates_applied) : updates.length,
    };
  }
  return {
    ok: false,
    type: 'orchestration_completion_track_batch',
    reason_code: 'orchestration_bridge_error',
  };
}

function loadTaskGroup(taskGroupId, options = {}) {
  const out = invokeOrchestration('taskgroup.load', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  const loaded = normalizeTaskGroupResponse(out, 'orchestration_taskgroup_load');
  if (loaded.ok) {
    return {
      ok: true,
      exists: Boolean(out && typeof out === 'object' ? out.exists : false),
      file_path: loaded.file_path || null,
      filePath: loaded.file_path || null,
      task_group: loaded.task_group || null,
      counts: loaded.counts,
    };
  }

  if (shouldFallbackForUnsupportedOp(loaded, 'taskgroup.load')) {
    const query = queryTaskGroup(taskGroupId, options);
    if (query.ok) {
      return {
        ok: true,
        exists: true,
        file_path: query.file_path,
        filePath: query.file_path,
        task_group: query.task_group,
        counts: query.counts || null,
      };
    }

    try {
      const filePath = taskGroupPath(taskGroupId, options);
      return {
        ok: true,
        exists: false,
        file_path: filePath,
        filePath,
        task_group: null,
        counts: null,
      };
    } catch (err) {
      return {
        ok: false,
        exists: false,
        type: loaded.type || 'orchestration_taskgroup_load',
        reason_code: String(err && err.message ? err.message : loaded.reason_code || 'orchestration_bridge_error'),
        file_path: null,
        filePath: null,
        task_group: null,
        counts: null,
      };
    }
  }

  return {
    ok: false,
    exists: false,
    type: loaded.type || 'orchestration_taskgroup_load',
    reason_code: loaded.reason_code || 'orchestration_bridge_error',
    file_path: null,
    filePath: null,
    task_group: null,
    counts: null,
  };
}

function saveTaskGroupLegacy(taskGroup, options = {}) {
  const ensured = ensureTaskGroup(taskGroup, options);
  if (!ensured.ok) return ensured;

  const taskGroupId = ensured.task_group && ensured.task_group.task_group_id
    ? ensured.task_group.task_group_id
    : taskGroup.task_group_id;

  const statusUpdate = batchUpdateAgentStatuses(
    taskGroupId,
    Array.isArray(taskGroup.agents) ? taskGroup.agents : [],
    options,
  );
  if (!statusUpdate.ok) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: statusUpdate.reason_code || 'batch_update_failed',
    };
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

function saveTaskGroup(taskGroup, options = {}) {
  if (!taskGroup || typeof taskGroup !== 'object' || Array.isArray(taskGroup)) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: 'invalid_taskgroup',
    };
  }

  const out = invokeOrchestration('taskgroup.save', {
    task_group: taskGroup,
    root_dir: options.rootDir || options.root_dir || undefined,
  });
  const saved = normalizeTaskGroupResponse(out, 'orchestration_taskgroup_save');
  if (saved.ok) {
    return {
      ok: true,
      type: 'orchestration_taskgroup_save',
      file_path: saved.file_path || null,
      filePath: saved.file_path || null,
      task_group: saved.task_group || null,
      counts: saved.counts || null,
    };
  }
  if (!shouldFallbackForUnsupportedOp(saved, 'taskgroup.save')) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_save',
      reason_code: saved.reason_code || 'orchestration_bridge_error',
    };
  }

  return saveTaskGroupLegacy(taskGroup, options);
}

function run(argv = process.argv.slice(2)) {
  return runTaskGroupCli(argv, {
    ensureTaskGroup,
    queryTaskGroup,
    listTaskGroupAgents,
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
  listTaskGroupAgents,
  run,
};
