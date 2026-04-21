#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson } = require('./cli_shared.ts');

function runTaskGroupCli(argv = [], api = {}) {
  const ensureTaskGroup = typeof api.ensureTaskGroup === 'function' ? api.ensureTaskGroup : null;
  const queryTaskGroup = typeof api.queryTaskGroup === 'function' ? api.queryTaskGroup : null;
  const updateAgentStatus = typeof api.updateAgentStatus === 'function' ? api.updateAgentStatus : null;
  if (!ensureTaskGroup || !queryTaskGroup || !updateAgentStatus) {
    return {
      ok: false,
      type: 'orchestration_taskgroup_command',
      reason_code: 'taskgroup_api_missing'
    };
  }

  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'status').trim().toLowerCase();
  const taskGroupId = String(
    parsed.flags['task-group-id']
      || parsed.flags.task_group_id
      || parsed.flags.id
      || parsed.positional[1]
      || ''
  ).trim().toLowerCase();

  if (command === 'create') {
    const agentsPayload = parseJson(parsed.flags['agents-json'] || parsed.flags.agents_json, [], 'invalid_agents_json');
    if (!agentsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_taskgroup_create',
        reason_code: agentsPayload.reason_code
      };
    }

    return ensureTaskGroup({
      task_group_id: taskGroupId,
      task_type: parsed.flags['task-type'] || parsed.flags.task_type || 'task',
      coordinator_session: parsed.flags['coordinator-session'] || parsed.flags.coordinator_session || '',
      agent_count: Number(parsed.flags['agent-count'] || parsed.flags.agent_count || 1),
      agents: agentsPayload.value,
      nonce: parsed.flags.nonce || ''
    });
  }

  if (command === 'status' || command === 'query') {
    if (!taskGroupId) {
      return {
        ok: false,
        type: 'orchestration_taskgroup_query',
        reason_code: 'missing_task_group_id'
      };
    }
    return queryTaskGroup(taskGroupId);
  }

  if (command === 'set-status') {
    if (!taskGroupId) {
      return {
        ok: false,
        type: 'orchestration_taskgroup_update_status',
        reason_code: 'missing_task_group_id'
      };
    }
    const agentId = parsed.flags['agent-id'] || parsed.flags.agent_id || '';
    const status = parsed.flags.status || '';
    const detailsPayload = parseJson(parsed.flags['details-json'] || parsed.flags.details_json, {}, 'invalid_details_json');
    if (!detailsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_taskgroup_update_status',
        reason_code: detailsPayload.reason_code
      };
    }
    return updateAgentStatus(taskGroupId, agentId, status, detailsPayload.value);
  }

  if (command === 'list-agents') {
    if (!taskGroupId) {
      return {
        ok: false,
        type: 'orchestration_taskgroup_list_agents',
        reason_code: 'missing_task_group_id'
      };
    }
    const query = queryTaskGroup(taskGroupId);
    if (!query.ok) return query;
    return {
      ok: true,
      type: 'orchestration_taskgroup_list_agents',
      task_group_id: query.task_group.task_group_id,
      agents: query.task_group.agents,
      counts: query.counts
    };
  }

  return {
    ok: false,
    type: 'orchestration_taskgroup_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['create', 'status', 'query', 'set-status', 'list-agents']
  };
}

module.exports = {
  runTaskGroupCli
};
