#!/usr/bin/env node
'use strict';

const fs = require('node:fs');
const path = require('node:path');
const { parseCliArgs } = require('./policy_validator.ts');

const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const NON_EXECUTING_COMMANDS = new Set(['status', 'verify', 'health', 'help']);
const DEFAULT_STATE_PATH = path.resolve(
  __dirname,
  '..',
  '..',
  'local',
  'state',
  'memory',
  'session_isolation.json'
);

function defaultState() {
  return {
    schema_version: '1.0',
    resources: {}
  };
}

function loadState(filePath = DEFAULT_STATE_PATH) {
  try {
    const raw = fs.readFileSync(filePath, 'utf8');
    const parsed = JSON.parse(raw);
    if (!parsed || typeof parsed !== 'object') return defaultState();
    if (!parsed.resources || typeof parsed.resources !== 'object') parsed.resources = {};
    if (!parsed.schema_version) parsed.schema_version = '1.0';
    return parsed;
  } catch {
    return defaultState();
  }
}

function saveState(state, filePath = DEFAULT_STATE_PATH) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(state, null, 2)}\n`);
}

function findSessionId(parsed, options = {}) {
  if (typeof options.sessionId === 'string' && options.sessionId.trim()) {
    return options.sessionId.trim();
  }
  const flags = parsed.flags || {};
  const fromFlags =
    flags['session-id']
    || flags.session_id
    || flags.session
    || flags['session-key']
    || flags.session_key;
  return String(fromFlags || '').trim();
}

function collectResourceKeys(parsed) {
  const out = [];
  const flags = parsed.flags || {};
  const resourceFlagNames = [
    'resource-id',
    'resource_id',
    'item-id',
    'item_id',
    'node-id',
    'node_id',
    'uid',
    'memory-id',
    'memory_id',
    'task-id',
    'task_id'
  ];
  for (const name of resourceFlagNames) {
    const value = String(flags[name] || '').trim();
    if (!value) continue;
    out.push(`${name}:${value}`);
  }
  return Array.from(new Set(out));
}

function validateSessionIsolation(args = [], options = {}) {
  const parsed = parseCliArgs(args);
  const command = String(options.command || parsed.positional[0] || 'status').trim().toLowerCase();
  const requireSession = options.requireSession !== false && !NON_EXECUTING_COMMANDS.has(command);
  const sessionId = findSessionId(parsed, options);

  if (requireSession && !sessionId) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'missing_session_id',
      command
    };
  }
  if (sessionId && !SESSION_ID_PATTERN.test(sessionId)) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'invalid_session_id',
      session_id: sessionId
    };
  }

  const resourceKeys = collectResourceKeys(parsed);
  if (!resourceKeys.length) {
    return {
      ok: true,
      type: 'memory_session_isolation',
      reason_code: 'no_resource_keys',
      command,
      session_id: sessionId || null
    };
  }

  const statePath = options.statePath || DEFAULT_STATE_PATH;
  const state = loadState(statePath);
  for (const key of resourceKeys) {
    const existing = state.resources[key];
    if (!existing || !existing.session_id) continue;
    if (sessionId && existing.session_id !== sessionId) {
      return {
        ok: false,
        type: 'memory_session_isolation',
        reason_code: 'cross_session_leak_blocked',
        resource_key: key,
        expected_session_id: existing.session_id,
        session_id: sessionId
      };
    }
  }

  if (options.persist !== false && sessionId) {
    const now = new Date().toISOString();
    for (const key of resourceKeys) {
      state.resources[key] = {
        session_id: sessionId,
        last_seen_at: now
      };
    }
    saveState(state, statePath);
  }

  return {
    ok: true,
    type: 'memory_session_isolation',
    reason_code: 'session_isolation_ok',
    session_id: sessionId || null,
    resource_key_count: resourceKeys.length
  };
}

function sessionFailureResult(validation, context = {}) {
  const payload = Object.assign(
    {
      ok: false,
      type: 'memory_session_isolation_reject',
      reason: validation && validation.reason_code ? validation.reason_code : 'session_isolation_failed',
      fail_closed: true
    },
    context && typeof context === 'object' ? context : {}
  );
  return {
    ok: false,
    status: 2,
    stdout: `${JSON.stringify(payload)}\n`,
    stderr: `memory_session_isolation_reject:${payload.reason}\n`,
    payload
  };
}

module.exports = {
  SESSION_ID_PATTERN,
  DEFAULT_STATE_PATH,
  loadState,
  saveState,
  validateSessionIsolation,
  sessionFailureResult
};
