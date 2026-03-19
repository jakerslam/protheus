#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../lib/rust_lane_bridge.ts');

const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const DEFAULT_STATE_PATH = path.resolve(
  __dirname,
  '..',
  '..',
  'local',
  'state',
  'memory',
  'session_isolation.json'
);

process.env.PROTHEUS_OPS_USE_PREBUILT = process.env.PROTHEUS_OPS_USE_PREBUILT || '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';
const bridge = createOpsLaneBridge(__dirname, 'session_isolation', 'memory-session-isolation-kernel');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function invoke(command, payload = {}, opts = {}) {
  const out = bridge.run([
    command,
    `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
  ]);
  const receipt = out && out.payload && typeof out.payload === 'object' ? out.payload : null;
  const payloadOut = receipt && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (out.status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (out && out.stderr ? String(out.stderr).trim() : `memory_session_isolation_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `memory_session_isolation_kernel_${command}_failed`);
    return { ok: false, error: message || `memory_session_isolation_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = out && out.stderr
      ? String(out.stderr).trim() || `memory_session_isolation_kernel_${command}_bridge_failed`
      : `memory_session_isolation_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
}

function loadState(filePath = DEFAULT_STATE_PATH) {
  const out = invoke('load-state', { state_path: filePath });
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
        resources: {}
      };
}

function saveState(state, filePath = DEFAULT_STATE_PATH) {
  const out = invoke('save-state', {
    state: state && typeof state === 'object' ? state : {},
    state_path: filePath
  });
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
      resources: {}
      };
}

function parseArgsToFlags(args = []) {
  const flags = {};
  for (const token of Array.isArray(args) ? args : []) {
    const value = String(token || '');
    if (!value.startsWith('--')) continue;
    const eq = value.indexOf('=');
    if (eq === -1) {
      flags[value.slice(2)] = '1';
    } else {
      flags[value.slice(2, eq)] = value.slice(eq + 1);
    }
  }
  return flags;
}

function loadLocalState(filePath) {
  try {
    if (fs.existsSync(filePath)) {
      const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
      if (parsed && typeof parsed === 'object' && parsed.resources && typeof parsed.resources === 'object') {
        return parsed;
      }
    }
  } catch {
    // fail closed to empty state snapshot
  }
  return {
    schema_version: '1.0',
    resources: {}
  };
}

function saveLocalState(state, filePath) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(state, null, 2)}\n`, 'utf8');
}

function localValidateSessionIsolation(args = [], options = {}) {
  const flags = parseArgsToFlags(args);
  const sessionId = typeof flags['session-id'] === 'string' ? flags['session-id'].trim() : '';
  const statePathCandidate = options && typeof options === 'object'
    ? (options.statePath || options.state_path || DEFAULT_STATE_PATH)
    : DEFAULT_STATE_PATH;
  const statePath = path.resolve(String(statePathCandidate || DEFAULT_STATE_PATH));
  if (!sessionId) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'missing_session_id'
    };
  }
  if (!SESSION_ID_PATTERN.test(sessionId)) {
    return {
      ok: false,
      type: 'memory_session_isolation',
      reason_code: 'invalid_session_id'
    };
  }
  const resourceId = typeof flags['resource-id'] === 'string' ? flags['resource-id'].trim() : '';
  if (resourceId) {
    const state = loadLocalState(statePath);
    const resources = state.resources && typeof state.resources === 'object' ? state.resources : {};
    const owner = typeof resources[resourceId] === 'string' ? resources[resourceId] : '';
    if (owner && owner !== sessionId) {
      return {
        ok: false,
        type: 'memory_session_isolation',
        reason_code: 'cross_session_leak_blocked'
      };
    }
    resources[resourceId] = sessionId;
    state.resources = resources;
    saveLocalState(state, statePath);
  }
  return {
    ok: true,
    type: 'memory_session_isolation',
    reason_code: 'session_isolation_ok'
  };
}

function validateSessionIsolation(args = [], options = {}) {
  return localValidateSessionIsolation(args, options);
}

function sessionFailureResult(validation, context = {}) {
  if (validation && typeof validation.reason_code === 'string' && validation.reason_code.trim()) {
    const reason = validation.reason_code.trim();
    return {
      ok: false,
      status: 2,
      stdout: `${JSON.stringify({
        ok: false,
        type: 'memory_session_isolation_reject',
        reason,
        fail_closed: true
      })}\n`,
      stderr: `memory_session_isolation_reject:${reason}\n`,
      payload: {
        ok: false,
        type: 'memory_session_isolation_reject',
        reason,
        fail_closed: true
      }
    };
  }
  const out = invoke('failure-result', {
    validation: validation && typeof validation === 'object' ? validation : {},
    context: context && typeof context === 'object' ? context : {}
  });
  return out.result && typeof out.result === 'object'
    ? out.result
    : {
        ok: false,
        status: 2,
        stdout: `${JSON.stringify({
          ok: false,
          type: 'memory_session_isolation_reject',
          reason: 'session_isolation_failed',
          fail_closed: true
        })}\n`,
        stderr: 'memory_session_isolation_reject:session_isolation_failed\n',
        payload: {
          ok: false,
          type: 'memory_session_isolation_reject',
          reason: 'session_isolation_failed',
          fail_closed: true
        }
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
