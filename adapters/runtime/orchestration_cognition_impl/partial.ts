#!/usr/bin/env node
'use strict';

const { parseArgs, parseJson, invokeOrchestration } = require('./core_bridge.ts');

function normalizeDecision(rawDecision, hasPartialResults) {
  const out = invokeOrchestration('partial.normalize_decision', {
    decision: String(rawDecision || ''),
    has_partial_results: Boolean(hasPartialResults),
  });
  return out && out.ok && out.decision ? String(out.decision) : 'retry';
}

function retrievePartialResults(input = {}) {
  return invokeOrchestration('partial.fetch', {
    ...(input && typeof input === 'object' ? input : {}),
  });
}

function extractPartialFromSessionEntry(entry) {
  const out = invokeOrchestration('partial.from_session_history', {
    session_history: [entry],
  });
  if (!out || !out.ok || out.source !== 'session_history') {
    return null;
  }
  return {
    partial_results: Array.isArray(out.findings_sofar) ? out.findings_sofar : [],
    items_completed: Number.isFinite(Number(out.items_completed)) ? Number(out.items_completed) : 0,
    checkpoint_path: out.checkpoint_path || null,
    source_session_id: out.source_session_id || null,
  };
}

function fromSessionHistory(history = []) {
  return invokeOrchestration('partial.from_session_history', {
    session_history: Array.isArray(history) ? history : [],
  });
}

function latestCheckpointFromScratchpad(taskId, options = {}) {
  return invokeOrchestration('partial.latest_checkpoint', {
    task_id: String(taskId || '').trim(),
    root_dir: options.rootDir || options.root_dir || undefined,
  });
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'fetch').trim().toLowerCase();
  if (command !== 'fetch' && command !== 'status') {
    return {
      ok: false,
      type: 'orchestration_partial_command',
      reason_code: `unsupported_command:${command}`,
      commands: ['fetch', 'status'],
    };
  }

  const taskId = String(
    parsed.flags['task-id']
      || parsed.flags.task_id
      || parsed.positional[1]
      || ''
  ).trim();

  const sessionPayload = parseJson(
    parsed.flags['session-history-json'] || parsed.flags.session_history_json,
    [],
    'invalid_session_history_json'
  );
  if (!sessionPayload.ok) {
    return {
      ok: false,
      type: 'orchestration_partial_retrieval',
      reason_code: sessionPayload.reason_code,
    };
  }

  return retrievePartialResults({
    task_id: taskId,
    session_history: sessionPayload.value,
    decision: parsed.flags.decision || '',
    root_dir: parsed.flags['root-dir'] || parsed.flags.root_dir || '',
  });
}

if (require.main === module) {
  const out = run(process.argv.slice(2));
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
  process.exit(out.ok ? 0 : 1);
}

module.exports = {
  normalizeDecision,
  extractPartialFromSessionEntry,
  fromSessionHistory,
  latestCheckpointFromScratchpad,
  retrievePartialResults,
  run,
};
