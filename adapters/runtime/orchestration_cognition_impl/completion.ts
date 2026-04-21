#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson, invokeOrchestration } = require('./core_bridge.ts');

function completionSummary(taskGroup) {
  const out = invokeOrchestration('completion.summarize', {
    task_group: taskGroup && typeof taskGroup === 'object' ? taskGroup : {},
    include_notification: false,
  });
  return out && out.ok && out.summary && typeof out.summary === 'object' ? out.summary : {};
}

function buildCompletionNotification(summary, taskGroup) {
  const out = invokeOrchestration('completion.summarize', {
    task_group: taskGroup && typeof taskGroup === 'object' ? taskGroup : {},
    include_notification: true,
  });
  return out && out.ok ? out.notification || null : null;
}

function ensureAndSummarize(taskGroupId, options = {}) {
  return invokeOrchestration('completion.status', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
}

function trackAgentCompletion(taskGroupId, update, options = {}) {
  return invokeOrchestration('completion.track', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    update: update && typeof update === 'object' ? update : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  });
}

function trackBatchCompletion(taskGroupId, updates = [], options = {}) {
  return invokeOrchestration('completion.batch', {
    task_group_id: String(taskGroupId || '').trim().toLowerCase(),
    updates: Array.isArray(updates) ? updates : [],
    root_dir: options.rootDir || options.root_dir || undefined,
  });
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
      reason_code: 'missing_task_group_id',
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
        reason_code: detailsPayload.reason_code,
      };
    }
    return trackAgentCompletion(taskGroupId, {
      agent_id: parsed.flags['agent-id'] || parsed.flags.agent_id || '',
      status: parsed.flags.status || '',
      details: detailsPayload.value,
    });
  }

  if (command === 'batch') {
    const updatesPayload = parseJson(parsed.flags['updates-json'] || parsed.flags.updates_json, [], 'invalid_updates_json');
    if (!updatesPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_completion_track_batch',
        reason_code: updatesPayload.reason_code,
      };
    }
    return trackBatchCompletion(taskGroupId, updatesPayload.value);
  }

  return {
    ok: false,
    type: 'orchestration_completion_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['status', 'track', 'batch'],
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
  run,
};
