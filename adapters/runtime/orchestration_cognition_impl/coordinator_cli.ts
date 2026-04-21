#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson } = require('./cli_shared.ts');

function runCoordinatorCli(argv = [], api = {}) {
  const runCoordinator = typeof api.runCoordinator === 'function' ? api.runCoordinator : null;
  const partitionWork = typeof api.partitionWork === 'function' ? api.partitionWork : null;
  const loadScratchpad = typeof api.loadScratchpad === 'function' ? api.loadScratchpad : null;
  const handleTimeout = typeof api.handleTimeout === 'function' ? api.handleTimeout : null;
  if (!runCoordinator || !partitionWork || !loadScratchpad || !handleTimeout) {
    return {
      ok: false,
      type: 'orchestration_coordinator',
      reason_code: 'coordinator_api_missing'
    };
  }

  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'run').trim().toLowerCase();

  if (command === 'run') {
    const taskId = String(parsed.flags['task-id'] || parsed.flags.task_id || parsed.positional[1] || '').trim();
    const auditId = String(parsed.flags['audit-id'] || parsed.flags.audit_id || '').trim();
    const rootDir = String(parsed.flags['root-dir'] || parsed.flags.root_dir || '').trim();
    const taskGroupId = String(parsed.flags['task-group-id'] || parsed.flags.task_group_id || '').trim();
    const taskType = String(parsed.flags['task-type'] || parsed.flags.task_type || 'audit').trim();
    const coordinatorSession = String(
      parsed.flags['coordinator-session'] || parsed.flags.coordinator_session || ''
    ).trim();
    const agentCount = Number(parsed.flags['agent-count'] || parsed.flags.agent_count || 1);

    const itemsPayload = parseJson(parsed.flags['items-json'] || parsed.flags.items_json, [], 'invalid_items_json');
    if (!itemsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_coordinator',
        reason_code: itemsPayload.reason_code
      };
    }

    const findingsPayload = parseJson(
      parsed.flags['findings-json'] || parsed.flags.findings_json,
      [],
      'invalid_findings_json'
    );
    if (!findingsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_coordinator',
        reason_code: findingsPayload.reason_code
      };
    }

    const scopesPayload = parseJson(parsed.flags['scopes-json'] || parsed.flags.scopes_json, [], 'invalid_scopes_json');
    if (!scopesPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_coordinator',
        reason_code: scopesPayload.reason_code
      };
    }

    const timeout = String(parsed.flags.timeout || '0').trim() === '1';
    if (timeout) {
      return handleTimeout(taskId, {
        processed_count: Number(parsed.flags.processed || 0),
        total_count: Array.isArray(itemsPayload.value) ? itemsPayload.value.length : 0,
        partial_results: Array.isArray(findingsPayload.value) ? findingsPayload.value : [],
        retry_count: Number(parsed.flags['retry-count'] || parsed.flags.retry_count || 0),
        now_ms: Date.now()
      }, rootDir ? { rootDir } : {});
    }

    return runCoordinator({
      task_id: taskId,
      audit_id: auditId,
      agent_count: agentCount,
      items: itemsPayload.value,
      findings: findingsPayload.value,
      scopes: scopesPayload.value,
      task_group_id: taskGroupId || undefined,
      task_type: taskType,
      coordinator_session: coordinatorSession || undefined,
      root_dir: rootDir || undefined
    });
  }

  if (command === 'partition') {
    const itemsPayload = parseJson(parsed.flags['items-json'] || parsed.flags.items_json, [], 'invalid_items_json');
    if (!itemsPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_partition',
        reason_code: itemsPayload.reason_code
      };
    }
    const scopesPayload = parseJson(parsed.flags['scopes-json'] || parsed.flags.scopes_json, [], 'invalid_scopes_json');
    if (!scopesPayload.ok) {
      return {
        ok: false,
        type: 'orchestration_partition',
        reason_code: scopesPayload.reason_code
      };
    }

    const agentCount = Number(parsed.flags['agent-count'] || parsed.flags.agent_count || 1);
    const out = partitionWork(itemsPayload.value, agentCount, scopesPayload.value);
    if (!out || typeof out.ok !== 'boolean') {
      return {
        ok: false,
        type: 'orchestration_partition',
        reason_code: 'orchestration_bridge_error'
      };
    }
    return out;
  }

  if (command === 'status') {
    const taskId = String(parsed.flags['task-id'] || parsed.flags.task_id || parsed.positional[1] || '').trim();
    if (!taskId) {
      return {
        ok: false,
        type: 'orchestration_coordinator_status',
        reason_code: 'missing_task_id'
      };
    }
    const loaded = loadScratchpad(taskId, {
      rootDir: parsed.flags['root-dir'] || parsed.flags.root_dir || undefined
    });
    return {
      ok: true,
      type: 'orchestration_coordinator_status',
      task_id: taskId,
      scratchpad_exists: loaded.exists,
      scratchpad_path: loaded.filePath,
      progress: loaded.scratchpad.progress || { processed: 0, total: 0 },
      finding_count: Array.isArray(loaded.scratchpad.findings) ? loaded.scratchpad.findings.length : 0,
      checkpoint_count: Array.isArray(loaded.scratchpad.checkpoints) ? loaded.scratchpad.checkpoints.length : 0
    };
  }

  return {
    ok: false,
    type: 'orchestration_coordinator',
    reason_code: `unsupported_command:${command}`,
    commands: ['run', 'partition', 'status']
  };
}

module.exports = {
  runCoordinatorCli
};
