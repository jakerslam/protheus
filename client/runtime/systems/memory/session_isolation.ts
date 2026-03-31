#!/usr/bin/env node
'use strict';

// Layer ownership: core/layer0/ops (authoritative)
// Thin TypeScript wrapper only.

const path = require('path');
const { spawnSync } = require('child_process');

const SESSION_ID_PATTERN = /^[A-Za-z0-9][A-Za-z0-9._:-]{2,127}$/;
const DEFAULT_STATE_PATH = 'client/runtime/local/state/memory/session_isolation.json';

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const OPS_WRAPPER = path.join(
  ROOT,
  'client',
  'runtime',
  'systems',
  'ops',
  'run_protheus_ops.ts'
);
const TS_ENTRYPOINT = path.join(ROOT, 'client', 'runtime', 'lib', 'ts_entrypoint.ts');

function encodeBase64(value) {
  return Buffer.from(String(value == null ? '' : value), 'utf8').toString('base64');
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}

function invoke(command, payload = {}, opts = {}) {
  const run = spawnSync(
    process.execPath,
    [
      TS_ENTRYPOINT,
      OPS_WRAPPER,
      'memory-session-isolation-kernel',
      command,
      `--payload-base64=${encodeBase64(JSON.stringify(payload || {}))}`
    ],
    {
      cwd: ROOT,
      encoding: 'utf8',
      env: { ...process.env }
    }
  );
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const receipt = parseLastJson(run.stdout);
  const payloadOut = receipt && typeof receipt === 'object'
    && receipt.payload && typeof receipt.payload === 'object'
    ? receipt.payload
    : receipt;
  if (status !== 0) {
    const message = payloadOut && typeof payloadOut.error === 'string'
      ? payloadOut.error
      : (run && run.stderr ? String(run.stderr).trim() : `memory_session_isolation_kernel_${command}_failed`);
    if (opts.throwOnError !== false) throw new Error(message || `memory_session_isolation_kernel_${command}_failed`);
    return { ok: false, error: message || `memory_session_isolation_kernel_${command}_failed` };
  }
  if (!payloadOut || typeof payloadOut !== 'object') {
    const message = run && run.stderr
      ? String(run.stderr).trim() || `memory_session_isolation_kernel_${command}_bridge_failed`
      : `memory_session_isolation_kernel_${command}_bridge_failed`;
    if (opts.throwOnError !== false) throw new Error(message);
    return { ok: false, error: message };
  }
  return payloadOut;
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
