#!/usr/bin/env node
'use strict';

const { appendCheckpoint, loadScratchpad, writeScratchpad } = require('./scratchpad.ts');

const ITEM_INTERVAL = 10;
const TIME_INTERVAL_MS = 120000;
const MAX_AUTO_RETRIES = 1;

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

function shouldCheckpoint(state, metrics, options = {}) {
  const itemInterval = Number.isFinite(Number(options.itemInterval))
    ? Number(options.itemInterval)
    : ITEM_INTERVAL;
  const timeIntervalMs = Number.isFinite(Number(options.timeIntervalMs))
    ? Number(options.timeIntervalMs)
    : TIME_INTERVAL_MS;
  const nowMs = Number.isFinite(Number(metrics.now_ms)) ? Number(metrics.now_ms) : Date.now();
  const processed = Number.isFinite(Number(metrics.processed_count))
    ? Number(metrics.processed_count)
    : 0;

  const checkpoints = Array.isArray(state.checkpoints) ? state.checkpoints : [];
  const last = checkpoints.length ? checkpoints[checkpoints.length - 1] : null;
  if (!last) {
    return processed > 0;
  }

  const lastProcessed = Number.isFinite(Number(last.processed_count))
    ? Number(last.processed_count)
    : 0;
  const lastNowMs = Number.isFinite(Number(last.now_ms))
    ? Number(last.now_ms)
    : nowMs;
  const itemDelta = processed - lastProcessed;
  const timeDelta = nowMs - lastNowMs;
  return itemDelta >= itemInterval || timeDelta >= timeIntervalMs;
}

function buildCheckpoint(taskId, metrics, reason) {
  return {
    task_id: taskId,
    reason,
    processed_count: Number.isFinite(Number(metrics.processed_count))
      ? Number(metrics.processed_count)
      : 0,
    total_count: Number.isFinite(Number(metrics.total_count))
      ? Number(metrics.total_count)
      : 0,
    now_ms: Number.isFinite(Number(metrics.now_ms)) ? Number(metrics.now_ms) : Date.now(),
    partial_results: Array.isArray(metrics.partial_results) ? metrics.partial_results : [],
    retry_count: Number.isFinite(Number(metrics.retry_count)) ? Number(metrics.retry_count) : 0
  };
}

function maybeCheckpoint(taskId, metrics, options = {}) {
  const loaded = loadScratchpad(taskId, options);
  const shouldWrite = shouldCheckpoint(loaded.scratchpad, metrics, options);
  if (!shouldWrite) {
    return {
      ok: true,
      type: 'orchestration_checkpoint_tick',
      checkpoint_written: false,
      task_id: taskId,
      checkpoint_path: loaded.filePath
    };
  }

  const checkpoint = buildCheckpoint(taskId, metrics, 'interval');
  const appended = appendCheckpoint(taskId, checkpoint, options);
  return {
    ok: appended.ok,
    type: 'orchestration_checkpoint_tick',
    checkpoint_written: appended.ok,
    task_id: taskId,
    checkpoint_path: appended.file_path,
    checkpoint
  };
}

function handleTimeout(taskId, metrics, options = {}) {
  const retryCount = Number.isFinite(Number(metrics.retry_count)) ? Number(metrics.retry_count) : 0;
  const retryAllowed = retryCount < MAX_AUTO_RETRIES;
  const checkpoint = buildCheckpoint(taskId, metrics, 'timeout');
  checkpoint.retry_allowed = retryAllowed;

  const appended = appendCheckpoint(taskId, checkpoint, options);
  if (appended.ok) {
    const loaded = loadScratchpad(taskId, options);
    const progress = Object.assign({}, loaded.scratchpad.progress || {});
    progress.processed = Number.isFinite(Number(metrics.processed_count))
      ? Number(metrics.processed_count)
      : progress.processed || 0;
    progress.total = Number.isFinite(Number(metrics.total_count))
      ? Number(metrics.total_count)
      : progress.total || 0;
    writeScratchpad(taskId, { progress }, options);
  }

  return {
    ok: appended.ok,
    type: 'orchestration_checkpoint_timeout',
    task_id: taskId,
    checkpoint_path: appended.file_path,
    checkpoint,
    partial_results: checkpoint.partial_results,
    retry_allowed: retryAllowed
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'tick').trim().toLowerCase();
  const taskId = String(parsed.flags['task-id'] || parsed.flags.task_id || parsed.positional[1] || '').trim();
  if (!taskId) {
    return {
      ok: false,
      type: 'orchestration_checkpoint_command',
      reason_code: 'missing_task_id'
    };
  }

  const metrics = {
    processed_count: Number(parsed.flags.processed || parsed.flags.processed_count || 0),
    total_count: Number(parsed.flags.total || parsed.flags.total_count || 0),
    now_ms: Number(parsed.flags['now-ms'] || parsed.flags.now_ms || Date.now()),
    retry_count: Number(parsed.flags['retry-count'] || parsed.flags.retry_count || 0),
    partial_results: []
  };

  if (parsed.flags['partial-results-json'] || parsed.flags.partial_results_json) {
    try {
      metrics.partial_results = JSON.parse(String(
        parsed.flags['partial-results-json'] || parsed.flags.partial_results_json
      ));
    } catch {
      return {
        ok: false,
        type: 'orchestration_checkpoint_command',
        reason_code: 'invalid_partial_results_json'
      };
    }
  }

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
    commands: ['tick', 'timeout']
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
  run
};
