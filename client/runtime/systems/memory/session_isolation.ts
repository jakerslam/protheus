#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const { invokeKernelPayload } = require('../../lib/protheus_kernel_bridge.ts');

const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const DEFAULT_STATE_PATH = 'client/runtime/local/state/memory/session_isolation.json';

function invoke(command, payload = {}, opts = {}) {
  return invokeKernelPayload(
    'memory-session-isolation-kernel',
    command,
    payload,
    {
      throwOnError: opts.throwOnError,
      fallbackError: `memory_session_isolation_kernel_${command}_bridge_failed`,
    }
  );
}

function loadState(filePath = DEFAULT_STATE_PATH) {
  const out = invoke(
    'load-state',
    {
      state_path: String(filePath || DEFAULT_STATE_PATH),
      statePath: String(filePath || DEFAULT_STATE_PATH)
    },
    { throwOnError: false }
  );
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
        resources: {}
      };
}

function saveState(state, filePath = DEFAULT_STATE_PATH) {
  const out = invoke(
    'save-state',
    {
      state: state && typeof state === 'object' ? state : {},
      state_path: String(filePath || DEFAULT_STATE_PATH),
      statePath: String(filePath || DEFAULT_STATE_PATH)
    },
    { throwOnError: false }
  );
  return out.state && typeof out.state === 'object'
    ? out.state
    : {
        schema_version: '1.0',
        resources: {}
      };
}

function validateSessionIsolation(args = [], options = {}) {
  const normalizedOptions = options && typeof options === 'object' ? { ...options } : {};
  const statePath = String(
    normalizedOptions.statePath || normalizedOptions.state_path || DEFAULT_STATE_PATH
  );
  if (!normalizedOptions.statePath) normalizedOptions.statePath = statePath;
  if (!normalizedOptions.state_path) normalizedOptions.state_path = statePath;

  const out = invoke(
    'validate',
    {
      args: Array.isArray(args) ? args.map((row) => String(row)) : [],
      options: normalizedOptions
    },
    { throwOnError: false }
  );
  return out.validation && typeof out.validation === 'object'
    ? out.validation
    : {
        ok: false,
        type: 'memory_session_isolation',
        reason_code: 'session_isolation_failed'
      };
}

function sessionFailureResult(validation, context = {}) {
  const out = invoke(
    'failure-result',
    {
      validation: validation && typeof validation === 'object' ? validation : {},
      context: context && typeof context === 'object' ? context : {}
    },
    { throwOnError: false }
  );
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
