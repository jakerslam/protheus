#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson, invokeOrchestration } = require('./core_bridge.ts');

const ITEM_INTERVAL = 10;
const TIME_INTERVAL_MS = 120000;
const MAX_AUTO_RETRIES = 1;

function shouldCheckpoint(state, metrics, options = {}) {
  const out = invokeOrchestration('checkpoint.should', {
    state: state && typeof state === 'object' ? state : {},
    metrics: metrics && typeof metrics === 'object' ? metrics : {},
    options: options && typeof options === 'object' ? options : {},
  });
  return Boolean(out && out.ok && out.should_checkpoint);
}

function maybeCheckpoint(taskId, metrics, options = {}) {
  return invokeOrchestration('checkpoint.tick', {
    task_id: String(taskId || '').trim(),
    metrics: metrics && typeof metrics === 'object' ? metrics : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  });
}

function handleTimeout(taskId, metrics, options = {}) {
  return invokeOrchestration('checkpoint.timeout', {
    task_id: String(taskId || '').trim(),
    metrics: metrics && typeof metrics === 'object' ? metrics : {},
    root_dir: options.rootDir || options.root_dir || undefined,
  });
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'tick').trim().toLowerCase();
  const taskId = String(parsed.flags['task-id'] || parsed.flags.task_id || parsed.positional[1] || '').trim();
  if (!taskId) {
    return {
      ok: false,
      type: 'orchestration_checkpoint_command',
      reason_code: 'missing_task_id',
    };
  }

  const partialPayload = parseJson(
    parsed.flags['partial-results-json'] || parsed.flags.partial_results_json,
    [],
    'invalid_partial_results_json',
  );
  if (!partialPayload.ok) {
    return {
      ok: false,
      type: 'orchestration_checkpoint_command',
      reason_code: partialPayload.reason_code,
    };
  }

  const metrics = {
    processed_count: Number(parsed.flags.processed || parsed.flags.processed_count || 0),
    total_count: Number(parsed.flags.total || parsed.flags.total_count || 0),
    now_ms: Number(parsed.flags['now-ms'] || parsed.flags.now_ms || Date.now()),
    retry_count: Number(parsed.flags['retry-count'] || parsed.flags.retry_count || 0),
    partial_results: Array.isArray(partialPayload.value) ? partialPayload.value : [],
  };

  if (command === 'tick') {
    return maybeCheckpoint(taskId, metrics);
  }
  if (command === 'timeout') {
    return handleTimeout(taskId, metrics);
  }

  return {
    ok: false,
    type: 'orchestration_checkpoint_command',
    reason_code: `unsupported_command:${command}`,
    commands: ['tick', 'timeout'],
  };
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  ITEM_INTERVAL,
  TIME_INTERVAL_MS,
  MAX_AUTO_RETRIES,
  shouldCheckpoint,
  maybeCheckpoint,
  handleTimeout,
  run,
};
