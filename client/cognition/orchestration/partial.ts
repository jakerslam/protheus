#!/usr/bin/env node
'use strict';

const { loadScratchpad } = require('./scratchpad.ts');

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

function normalizeDecision(rawDecision, hasPartialResults) {
  const value = String(rawDecision || '').trim().toLowerCase();
  if (value === 'retry' || value === 'continue' || value === 'abort') return value;
  return hasPartialResults ? 'continue' : 'retry';
}

function extractPartialFromSessionEntry(entry) {
  if (!entry || typeof entry !== 'object' || Array.isArray(entry)) return null;

  const candidates = [
    entry.partial_results,
    entry.partialResults,
    entry.partial,
    entry.findings,
    entry.result && entry.result.partial_results,
    entry.result && entry.result.findings,
    entry.output && entry.output.partial_results,
    entry.output && entry.output.findings,
    entry.payload && entry.payload.partial_results,
    entry.payload && entry.payload.findings
  ];

  for (const candidate of candidates) {
    if (!Array.isArray(candidate) || candidate.length === 0) continue;
    return {
      partial_results: candidate,
      items_completed: Number.isFinite(Number(entry.items_completed))
        ? Number(entry.items_completed)
        : Number.isFinite(Number(entry.processed_count))
          ? Number(entry.processed_count)
          : candidate.length,
      checkpoint_path: entry.checkpoint_path || entry.checkpointPath || null,
      source_session_id: entry.session_id || entry.sessionId || null
    };
  }

  return null;
}

function fromSessionHistory(history = []) {
  const source = Array.isArray(history) ? history : [];
  for (let index = source.length - 1; index >= 0; index -= 1) {
    const extracted = extractPartialFromSessionEntry(source[index]);
    if (extracted) {
      return {
        ok: true,
        type: 'orchestration_partial_from_session_history',
        source: 'session_history',
        items_completed: extracted.items_completed,
        findings_sofar: extracted.partial_results,
        checkpoint_path: extracted.checkpoint_path,
        source_session_id: extracted.source_session_id
      };
    }
  }
  return {
    ok: false,
    type: 'orchestration_partial_from_session_history',
    reason_code: 'session_history_no_partial_results'
  };
}

function latestCheckpointFromScratchpad(taskId, options = {}) {
  const loaded = loadScratchpad(taskId, options);
  if (!loaded || !loaded.scratchpad || !Array.isArray(loaded.scratchpad.checkpoints)) {
    return {
      ok: false,
      type: 'orchestration_partial_checkpoint_fallback',
      reason_code: 'scratchpad_missing',
      task_id: taskId,
      checkpoint_path: loaded ? loaded.filePath : null
    };
  }

  const checkpoints = loaded.scratchpad.checkpoints;
  const latest = checkpoints.length ? checkpoints[checkpoints.length - 1] : null;
  if (!latest || !Array.isArray(latest.partial_results) || latest.partial_results.length === 0) {
    return {
      ok: false,
      type: 'orchestration_partial_checkpoint_fallback',
      reason_code: 'checkpoint_no_partial_results',
      task_id: taskId,
      checkpoint_path: loaded.filePath
    };
  }

  return {
    ok: true,
    type: 'orchestration_partial_checkpoint_fallback',
    source: 'checkpoint',
    task_id: taskId,
    checkpoint_path: loaded.filePath,
    items_completed: Number.isFinite(Number(latest.processed_count))
      ? Number(latest.processed_count)
      : latest.partial_results.length,
    findings_sofar: latest.partial_results,
    retry_allowed: Boolean(latest.retry_allowed)
  };
}

function retrievePartialResults(input = {}) {
  const taskId = String(input.task_id || input.taskId || '').trim();
  if (!taskId) {
    return {
      ok: false,
      type: 'orchestration_partial_retrieval',
      reason_code: 'missing_task_id'
    };
  }

  const sessionHistory = Array.isArray(input.session_history || input.sessionHistory)
    ? input.session_history || input.sessionHistory
    : [];

  const fromSessions = fromSessionHistory(sessionHistory);
  if (fromSessions.ok) {
    const decision = normalizeDecision(input.decision, true);
    return {
      ok: true,
      type: 'orchestration_partial_retrieval',
      source: fromSessions.source,
      task_id: taskId,
      items_completed: fromSessions.items_completed,
      findings_sofar: fromSessions.findings_sofar,
      checkpoint_path: fromSessions.checkpoint_path,
      source_session_id: fromSessions.source_session_id,
      decision
    };
  }

  const checkpointFallback = latestCheckpointFromScratchpad(taskId, {
    rootDir: input.root_dir || input.rootDir
  });
  if (!checkpointFallback.ok) {
    return {
      ok: false,
      type: 'orchestration_partial_retrieval',
      reason_code: 'partial_results_unavailable',
      task_id: taskId,
      attempted_sources: ['session_history', 'checkpoint'],
      checkpoint_reason: checkpointFallback.reason_code
    };
  }

  const decision = normalizeDecision(input.decision, true);
  return {
    ok: true,
    type: 'orchestration_partial_retrieval',
    source: checkpointFallback.source,
    task_id: taskId,
    items_completed: checkpointFallback.items_completed,
    findings_sofar: checkpointFallback.findings_sofar,
    checkpoint_path: checkpointFallback.checkpoint_path,
    retry_allowed: checkpointFallback.retry_allowed,
    decision
  };
}

function run(argv = process.argv.slice(2)) {
  const parsed = parseArgs(argv);
  const command = String(parsed.positional[0] || 'fetch').trim().toLowerCase();
  if (command !== 'fetch' && command !== 'status') {
    return {
      ok: false,
      type: 'orchestration_partial_command',
      reason_code: `unsupported_command:${command}`,
      commands: ['fetch', 'status']
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
      reason_code: sessionPayload.reason_code
    };
  }

  return retrievePartialResults({
    task_id: taskId,
    session_history: sessionPayload.value,
    decision: parsed.flags.decision || '',
    root_dir: parsed.flags['root-dir'] || parsed.flags.root_dir || ''
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
  run
};
