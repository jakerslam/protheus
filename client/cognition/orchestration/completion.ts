#!/usr/bin/env node
'use strict';

const {
  ensureTaskGroup,
  queryTaskGroup,
  updateAgentStatus,
  statusCounts,
  TERMINAL_AGENT_STATUSES
} = require('./taskgroup.ts');

function parseArgs(argv = []) {
  const positional = [];
  const flags = {};
  for (const raw of Array.isArray(argv) ? argv : []) {
    const token = String(raw || '').trim();
    if (!token) continue;
    if (token.startsWith('--')) {
      const body = token.slice(2);
      const eq = body.indexOf('=');
      if (eq >= 0) flags[body.slice(0, eq)] = body.slice(eq + 1);
      else flags[body] = '1';
      continue;
    }
    positional.push(token);
  }
  return { positional, flags };
}

function parseJson(raw, fallback, reasonCode) {
  if (raw == null || String(raw).trim() === '') return { ok: true, value: fallback };
  try {
    return { ok: true, value: JSON.parse(String(raw)) };
  } catch {
    return { ok: false, reason_code: reasonCode };
  }
}

function partialCountFromGroup(group) {
  const agents = Array.isArray(group && group.agents) ? group.agents : [];
  let total = 0;
  for (const agent of agents) {
    const details = agent && typeof agent === 'object' ? agent.details : null;
    if (!details || typeof details !== 'object') continue;
    const count = Number.isFinite(Number(details.partial_results_count))
      ? Number(details.partial_results_count)
      : Array.isArray(details.partial_results)
        ? details.partial_results.length
        : 0;
    if (count > 0) total += 1;
  }
  return total;
}

function completionSummary(taskGroup) {
  const counts = statusCounts(taskGroup);
  const terminalTotal = counts.done + counts.failed + counts.timeout;
  const complete = counts.total > 0 && terminalTotal === counts.total;
  return {
    task_group_id: String(taskGroup && taskGroup.task_group_id ? taskGroup.task_group_id : '').trim().toLowerCase(),
    status: String(taskGroup && taskGroup.status ? taskGroup.status : '').trim() || 'pending',
    completed_count: counts.done,
    failed_count: counts.failed,
    timeout_count: counts.timeout,
    pending_count: counts.pending,
    running_count: counts.running,
    partial_count: partialCountFromGroup(taskGroup),
    total_count: counts.total,
    complete,
    counts
  };
}

function buildCompletionNotification(summary, taskGroup) {
  return {
    type: 'orchestration_completion_notification',
    task_group_id: summary.task_group_id,
    coordinator_session: taskGroup && taskGroup.coordinator_session ? taskGroup.coordinator_session : null,
    status: summary.status,
    completed_count: summary.completed_count,
    failed_count: summary.failed_count,
    timeout_count: summary.timeout_count,
    partial_count: summary.partial_count,
    total_count: summary.total_count,
    generated_at: new Date().toISOString()
  };
}

function ensureAndSummarize(taskGroupId, options = {}) {
  const ensured = ensureTaskGroup({ task_group_id: taskGroupId }, options);
  if (!ensured.ok) return ensured;
  const summary = completionSummary(ensured.task_group);
  return {
    ok: true,
    type: 'orchestration_completion_summary',
    task_group: ensured.task_group,
    summary,
    notification: summary.complete ? buildCompletionNotification(summary, ensured.task_group) : null
  };
}

function trackAgentCompletion(taskGroupId, update, options = {}) {
  const normalized = update && typeof update === 'object' && !Array.isArray(update) ? update : {};
  const agentId = String(normalized.agent_id || normalized.agentId || '').trim();
  const status = String(normalized.status || '').trim().toLowerCase();
  if (!agentId) {
    return {
      ok: false,
      type: 'orchestration_completion_track',
      reason_code: 'missing_agent_id'
    };
  }
  if (!TERMINAL_AGENT_STATUSES.has(status) && status !== 'pending' && status !== 'running') {
    return {
      ok: false,
      type: 'orchestration_completion_track',
      reason_code: `invalid_agent_status:${status || '<empty>'}`
    };
  }

  const details = normalized.details && typeof normalized.details === 'object' && !Array.isArray(normalized.details)
    ? normalized.details
    : {};

  const updated = updateAgentStatus(taskGroupId, agentId, status, details, options);
  if (!updated.ok) return updated;

  const summary = completionSummary(updated.task_group);
  return {
    ok: true,
    type: 'orchestration_completion_track',
    task_group: updated.task_group,
    summary,
    notification: summary.complete ? buildCompletionNotification(summary, updated.task_group) : null
  };
}

function trackBatchCompletion(taskGroupId, updates = [], options = {}) {
  const source = Array.isArray(updates) ? updates : [];
  const results = [];

  for (const update of source) {
    const tracked = trackAgentCompletion(taskGroupId, update, options);
    if (!tracked.ok) {
      return {
        ok: false,
        type: 'orchestration_completion_track_batch',
        reason_code: tracked.reason_code || 'batch_update_failed',
        failed_update: update
      };
    }
    results.push({
      agent_id: String(update && (update.agent_id || update.agentId) ? update.agent_id || update.agentId : '').trim(),
      status: String(update && update.status ? update.status : '').trim().toLowerCase(),
      summary: tracked.summary
    });
  }

  const query = queryTaskGroup(taskGroupId, options);
  if (!query.ok) return query;
  const summary = completionSummary(query.task_group);

  return {
    ok: true,
    type: 'orchestration_completion_track_batch',
    task_group: query.task_group,
    summary,
    updates_applied: results.length,
    updates: results,
    notification: summary.complete ? buildCompletionNotification(summary, query.task_group) : null
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'status').trim().toLowerCase();
  const taskGroupId = String(
    parsed.flags['task-group-id']
      || parsed.flags.task_group_id
      || parsed.flags.id
      || parsed.positional[1]
      || ''
  ).trim().toLowerCase();

  if (!taskGroupId) {
    return {
      ok: false,
      type: 'orchestration_completion_command',
      reason_code: 'missing_task_group_id'
    };
  }

  if (command === 'status') {
    return ensureAndSummarize(taskGroupId);
  }

  if (command === 'track') {
    const detailsPayload = parseJson(parsed.flags['details-json'] || parsed.flags.details_json, {}, 'invalid_details_json');
    if (!detailsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_completion_track',
        reason_code: detailsPayload.reason_code
      };
    }
    return trackAgentCompletion(taskGroupId, {
      agent_id: parsed.flags['agent-id'] || parsed.flags.agent_id || '',
      status: parsed.flags.status || '',
      details: detailsPayload.value
    });
  }

  if (command === 'batch') {
    const updatesPayload = parseJson(parsed.flags['updates-json'] || parsed.flags.updates_json, [], 'invalid_updates_json');
    if (!updatesPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_completion_track_batch',
        reason_code: updatesPayload.reason_code
      };
    }
    return trackBatchCompletion(taskGroupId, updatesPayload.value);
  }

  return {
    ok: false,
    type: 'orchestration_completion_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['status', 'track', 'batch']
  };
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  completionSummary,
  buildCompletionNotification,
  ensureAndSummarize,
  trackAgentCompletion,
  trackBatchCompletion,
  run
};
