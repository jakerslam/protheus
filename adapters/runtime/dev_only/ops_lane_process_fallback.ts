'use strict';

// process_fallback_dev_only

const { spawnSync } = require('child_process');

function runLocalOpsDomainProcessFallback(options) {
  const root = options && options.root ? options.root : process.cwd();
  const domain = options && options.domain ? options.domain : '';
  const passArgs = Array.isArray(options && options.passArgs) ? options.passArgs : [];
  const cliMode = options && options.cliMode === true;
  const inheritStdio = options && options.inheritStdio === true;
  const resolved = options && options.resolved ? options.resolved : { command: '', args: [] };
  const parseTimeoutMs = options.parseTimeoutMs;
  const defaultEnv = options.defaultEnv;
  const deferOnHostStallEnabled = options.deferOnHostStallEnabled;
  const isTimeoutLikeSpawnError = options.isTimeoutLikeSpawnError;
  const normalizeStatus = options.normalizeStatus;
  const parseJsonPayload = options.parseJsonPayload;

  const commandArgs = resolved.args.concat(passArgs);
  const timeoutMs = parseTimeoutMs('PROTHEUS_OPS_LOCAL_TIMEOUT_MS', 45000);
  const run = spawnSync(resolved.command, commandArgs, {
    cwd: root,
    encoding: 'utf8',
    env: defaultEnv(),
    stdio: cliMode && inheritStdio ? 'inherit' : undefined,
    timeout: timeoutMs,
    maxBuffer: 1024 * 1024 * 4,
  });
  if (deferOnHostStallEnabled() && isTimeoutLikeSpawnError(run.error)) {
    const payload = {
      ok: true,
      type: 'ops_domain_deferred_host_stall',
      reason_code: 'deferred_host_stall',
      raw_error_code: String(run.error.code || ''),
      domain,
      timeout_ms: timeoutMs,
    };
    return {
      ok: true,
      status: 0,
      stdout: cliMode && inheritStdio ? '' : `${JSON.stringify(payload)}\n`,
      stderr: String(run.error && run.error.message ? run.error.message : run.error),
      payload,
      rust_command: resolved.command,
      rust_args: [resolved.command, ...commandArgs],
      timeout_ms: timeoutMs,
      routed_via: 'core_local',
      deferred_host_stall: true,
    };
  }
  const status = run.error ? 1 : normalizeStatus(run.status);
  const stdout = run.stdout || '';
  const stderr = `${run.stderr || ''}${run.error ? `\n${String(run.error && run.error.message ? run.error.message : run.error)}` : ''}`;
  const payload = cliMode && inheritStdio ? null : parseJsonPayload(stdout);
  if (!payload && run.error) {
    return {
      ok: false,
      status,
      stdout,
      stderr,
      payload: {
        ok: false,
        type: 'ops_domain_spawn_error',
        reason: String(run.error && run.error.message ? run.error.message : run.error),
        raw_error_code: String(run.error.code || ''),
        domain,
      },
      error: run.error,
      rust_command: resolved.command,
      rust_args: [resolved.command, ...commandArgs],
      timeout_ms: timeoutMs,
      routed_via: 'core_local',
    };
  }
  return {
    ok: status === 0,
    status,
    stdout,
    stderr,
    payload,
    error: run.error || null,
    rust_command: resolved.command,
    rust_args: [resolved.command, ...commandArgs],
    timeout_ms: timeoutMs,
    routed_via: 'core_local',
  };
}

function shouldRetryProcessFallbackWithCargo(result) {
  if (!result || result.status === 0) return false;
  const rawErrorCode = String(
    (result.payload && result.payload.raw_error_code) || (result.error && result.error.code) || ''
  ).toLowerCase();
  if (rawErrorCode === 'enoent' || rawErrorCode === 'eacces') {
    return true;
  }
  const reason = String(
    (result.payload && result.payload.reason) ||
      (result.payload && result.payload.error) ||
      result.stderr ||
      ''
  ).toLowerCase();
  return reason.includes('unknown_domain') || reason.includes('unknown_command');
}

module.exports = {
  runLocalOpsDomainProcessFallback,
  shouldRetryProcessFallbackWithCargo,
};
