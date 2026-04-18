'use strict';

const crypto = require('node:crypto');
const { createOpsLaneBridge } = require('./rust_lane_bridge.ts');

const DEFAULT_MAX_ARG_LEN = 512;
const DEFAULT_MAX_ARGS = 64;
const RETRYABLE_ERROR_RE =
  /429|rate\s*limit|timeout|timed\s*out|deadline|connect|reset|closed|temporar(?:y|ily)|unavailable|retry/i;
const UNAVAILABLE_ERROR_RE =
  /enoent|not\s+found|missing|spawn\s+\S+\s+eacces|permission\s+denied/i;
const TIMEOUT_ERROR_RE = /timeout|timed\s*out|deadline/i;
const MUTATING_ACTIONS = new Set([
  'apply',
  'append',
  'delete',
  'edit',
  'kill',
  'patch',
  'restart',
  'run',
  'send',
  'start',
  'stop',
  'submit',
  'update',
  'write'
]);
const READ_ONLY_ACTIONS = new Set([
  'check',
  'fetch',
  'help',
  'inspect',
  'list',
  'poll',
  'probe',
  'query',
  'read',
  'search',
  'show',
  'status',
  'view'
]);

function stableStringify(value) {
  if (value === null || typeof value !== 'object') {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  }
  const keys = Object.keys(value).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
}

function normalizeReceiptHash(payload) {
  const clone = Object.assign({}, payload);
  delete clone.receipt_hash;
  return crypto.createHash('sha256').update(stableStringify(clone)).digest('hex');
}

function withReceiptHash(payload) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    return payload;
  }
  if (typeof payload.receipt_hash === 'string' && payload.receipt_hash.trim()) {
    return payload;
  }
  return Object.assign({}, payload, { receipt_hash: normalizeReceiptHash(payload) });
}

function sanitizeBridgeArg(value, maxArgLen = DEFAULT_MAX_ARG_LEN) {
  return String(value == null ? '' : value)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/[\r\n\t]+/g, ' ')
    .replace(/[^\x20-\x7E]+/g, '')
    .trim()
    .slice(0, maxArgLen);
}

function normalizeErrorText(value) {
  if (typeof value === 'string') {
    const trimmed = value.trim();
    return trimmed || '';
  }
  if (value == null) {
    return '';
  }
  if (value instanceof Error) {
    return normalizeErrorText(value.message || value.name);
  }
  if (typeof value === 'number' || typeof value === 'boolean' || typeof value === 'bigint') {
    return String(value);
  }
  try {
    return normalizeErrorText(JSON.stringify(value));
  } catch {
    return '';
  }
}

function resolvePositiveIntegerOption(value, fallback, min, max) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed)) {
    return fallback;
  }
  const normalized = Math.trunc(parsed);
  if (normalized < min || normalized > max) {
    return fallback;
  }
  return normalized;
}

function toFiniteNumber(value) {
  if (typeof value === 'number' && Number.isFinite(value)) {
    return value;
  }
  if (typeof value === 'string') {
    const parsed = Number.parseFloat(value);
    if (Number.isFinite(parsed)) {
      return parsed;
    }
  }
  return undefined;
}

function extractJsonObjectCandidates(raw) {
  const candidates = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;

  for (let index = 0; index < raw.length; index += 1) {
    const ch = raw[index] || '';
    if (escaped) {
      escaped = false;
      continue;
    }
    if (ch === '\\') {
      if (inString) escaped = true;
      continue;
    }
    if (ch === '"') {
      inString = !inString;
      continue;
    }
    if (inString) continue;
    if (ch === '{') {
      if (depth === 0) start = index;
      depth += 1;
      continue;
    }
    if (ch === '}' && depth > 0) {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        candidates.push(raw.slice(start, index + 1));
        start = -1;
      }
    }
  }

  return candidates;
}

function parseJsonRecordCandidates(raw) {
  const records = [];
  const trimmed = normalizeErrorText(raw);
  if (!trimmed) return records;

  try {
    const parsed = JSON.parse(trimmed);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      records.push(parsed);
      return records;
    }
  } catch {
    // mixed output path below
  }

  for (const fragment of extractJsonObjectCandidates(trimmed)) {
    try {
      const parsed = JSON.parse(fragment);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        records.push(parsed);
      }
    } catch {
      // ignore malformed fragment
    }
  }

  return records;
}

function readNestedErrorMessage(record) {
  if (!record || typeof record !== 'object' || Array.isArray(record)) {
    return undefined;
  }

  if (typeof record.message === 'string' && record.message.trim()) {
    return record.message.trim();
  }
  if (typeof record.error === 'string' && record.error.trim()) {
    return record.error.trim();
  }
  if (record.error && typeof record.error === 'object') {
    const nested = readNestedErrorMessage(record.error);
    if (nested) return nested;
  }
  if (record.details && typeof record.details === 'object') {
    const nested = readNestedErrorMessage(record.details);
    if (nested) return nested;
  }
  return undefined;
}

function extractRetryAfterMs(record) {
  if (!record || typeof record !== 'object' || Array.isArray(record)) {
    return undefined;
  }

  const direct =
    toFiniteNumber(record.retry_after_ms) ??
    toFiniteNumber(record.retryAfterMs) ??
    toFiniteNumber(record.retry_after) ??
    toFiniteNumber(record.retryAfter);

  if (direct !== undefined) {
    if (direct <= 0) return undefined;
    return direct <= 300 ? Math.trunc(direct * 1000) : Math.trunc(direct);
  }

  const nestedCandidates = [record.parameters, record.response, record.error, record.details];
  for (const nested of nestedCandidates) {
    if (!nested || typeof nested !== 'object' || Array.isArray(nested)) continue;
    const nestedValue = extractRetryAfterMs(nested);
    if (nestedValue !== undefined) {
      return nestedValue;
    }
  }

  return undefined;
}

function resolveStructuredErrorDetails(value) {
  if (value && typeof value === 'object' && !Array.isArray(value)) {
    const text = normalizeErrorText(readNestedErrorMessage(value) || value.message || value.error);
    return { text, record: value };
  }

  const raw = normalizeErrorText(value);
  if (!raw) {
    return { text: '', record: undefined };
  }

  const parsedRecords = parseJsonRecordCandidates(raw);
  for (const record of parsedRecords) {
    const nested = normalizeErrorText(readNestedErrorMessage(record));
    if (nested) {
      return { text: nested, record };
    }
  }

  return { text: raw, record: parsedRecords[0] };
}

function normalizeBridgeArgsDetailed(args, maxArgLen, maxArgs) {
  const raw = Array.isArray(args) ? args : [];
  const sanitized = raw.map((arg) => sanitizeBridgeArg(arg, maxArgLen)).filter(Boolean);
  return {
    passthrough: sanitized.slice(0, maxArgs),
    raw_count: raw.length,
    sanitized_count: sanitized.length,
    dropped_count: Math.max(0, raw.length - Math.min(sanitized.length, maxArgs)),
    truncated: sanitized.length > maxArgs || sanitized.length !== raw.length,
  };
}

function normalizeExitCode(out) {
  return Number.isFinite(Number(out && out.status)) ? Number(out && out.status) : 1;
}

function detectMutationLikely(passthroughArgs) {
  const action = String((Array.isArray(passthroughArgs) && passthroughArgs[0]) || '')
    .trim()
    .toLowerCase();
  if (!action) {
    return false;
  }
  if (READ_ONLY_ACTIONS.has(action)) {
    return false;
  }
  if (MUTATING_ACTIONS.has(action)) {
    return true;
  }
  return (
    action.startsWith('set_') ||
    action.startsWith('update_') ||
    action.startsWith('delete_') ||
    action.startsWith('patch_')
  );
}

function computeAttemptSignature(systemId, passthroughArgs) {
  return crypto
    .createHash('sha256')
    .update(
      stableStringify({
        system_id: String(systemId || ''),
        args: Array.isArray(passthroughArgs) ? passthroughArgs : []
      })
    )
    .digest('hex');
}

function classifyBridgeError({ status, error, errorCode }) {
  const normalizedErrorCode = normalizeErrorText(errorCode).toLowerCase();
  const normalizedError = normalizeErrorText(error).toLowerCase();

  if (normalizedErrorCode === 'bridge_no_output') {
    return 'transport_no_output';
  }
  if (status === 124 || TIMEOUT_ERROR_RE.test(normalizedError)) {
    return 'timeout';
  }
  if (status === 126 || status === 127 || UNAVAILABLE_ERROR_RE.test(normalizedError)) {
    return 'transport_unavailable';
  }
  if (RETRYABLE_ERROR_RE.test(normalizedError)) {
    return 'transient';
  }
  if (normalizedError || status !== 0) {
    return 'execution_error';
  }
  return 'none';
}

function inferBridgeErrorCode(errorClass, status, explicitErrorCode) {
  const normalizedExplicit = normalizeErrorText(explicitErrorCode);
  if (normalizedExplicit) {
    return normalizedExplicit;
  }
  if (errorClass === 'timeout' || status === 124) {
    return 'bridge_timeout';
  }
  if (errorClass === 'transport_unavailable') {
    return 'bridge_transport_unavailable';
  }
  if (errorClass === 'transient') {
    return 'bridge_transient_failure';
  }
  if (errorClass === 'transport_no_output') {
    return 'bridge_no_output';
  }
  if (errorClass === 'execution_error') {
    return 'bridge_execution_failed';
  }
  return undefined;
}

function retryHintsForErrorClass(errorClass, retryAfterMs) {
  if (errorClass === 'timeout' || errorClass === 'transient') {
    const minDelay = retryAfterMs !== undefined ? Math.max(200, retryAfterMs) : 400;
    const maxDelay = retryAfterMs !== undefined ? Math.max(minDelay, retryAfterMs) : 5000;
    return {
      recommended: true,
      strategy: 'bounded_backoff',
      lane: 'same_lane_retry',
      attempts: 2,
      min_delay_ms: minDelay,
      max_delay_ms: maxDelay,
      jitter: retryAfterMs !== undefined ? 0.0 : 0.1,
      ...(retryAfterMs !== undefined ? { retry_after_ms: retryAfterMs } : {})
    };
  }
  if (errorClass === 'transport_no_output') {
    return {
      recommended: true,
      strategy: 'quick_retry',
      lane: 'same_lane_retry',
      attempts: 1,
      min_delay_ms: 250,
      max_delay_ms: 1000,
      jitter: 0.0
    };
  }
  if (errorClass === 'transport_unavailable') {
    return {
      recommended: false,
      strategy: 'manual_recovery',
      lane: 'operator_fix'
    };
  }
  return {
    recommended: false,
    strategy: 'none',
    lane: 'none'
  };
}

function buildToolErrorSummary(type, errorText, errorClass, attemptSignature, mutationLikely) {
  if (!errorText && errorClass === 'none') {
    return undefined;
  }
  return {
    toolName: String(type || ''),
    meta: errorClass !== 'none' ? errorClass : undefined,
    error: errorText || undefined,
    timedOut: errorClass === 'timeout',
    mutatingAction: Boolean(mutationLikely),
    actionFingerprint: attemptSignature
  };
}

function buildErrorContext({
  type,
  status,
  error,
  errorCode,
  attemptSignature,
  mutationLikely,
  retryAfterMs,
}) {
  const normalizedError = normalizeErrorText(error) || normalizeErrorText(errorCode);
  const errorClass = classifyBridgeError({
    status,
    error: normalizedError,
    errorCode
  });
  return {
    errorText: normalizedError,
    errorClass,
    errorCode: inferBridgeErrorCode(errorClass, status, errorCode),
    retry: retryHintsForErrorClass(errorClass, retryAfterMs),
    toolErrorSummary: buildToolErrorSummary(
      type,
      normalizedError,
      errorClass,
      attemptSignature,
      mutationLikely
    )
  };
}

function enrichObjectPayload(payload, context) {
  const enriched = Object.assign({}, payload);
  if (typeof enriched.attempt_signature !== 'string' || !enriched.attempt_signature.trim()) {
    enriched.attempt_signature = context.attemptSignature;
  }
  if (enriched.status === undefined) {
    enriched.status = context.status;
  }
  if (!enriched.arg_policy && context.argPolicy) {
    enriched.arg_policy = context.argPolicy;
  }
  if (typeof enriched.mutation_likely !== 'boolean') {
    enriched.mutation_likely = Boolean(context.mutationLikely);
  }

  const shouldAnnotate = enriched.ok === false || Boolean(context.errorText) || context.status !== 0;
  if (!shouldAnnotate) {
    return withReceiptHash(enriched);
  }

  const errorContext = buildErrorContext({
    type: context.type,
    status: context.status,
    error: context.errorText,
    errorCode: context.errorCode,
    attemptSignature: context.attemptSignature,
    mutationLikely: context.mutationLikely,
    retryAfterMs: context.retryAfterMs
  });

  if (
    errorContext.errorClass !== 'none' &&
    (typeof enriched.error_class !== 'string' || !enriched.error_class.trim())
  ) {
    enriched.error_class = errorContext.errorClass;
  }
  if (typeof enriched.error_code !== 'string' && errorContext.errorCode) {
    enriched.error_code = errorContext.errorCode;
  }
  if (!enriched.retry && errorContext.retry) {
    enriched.retry = errorContext.retry;
  }
  if (!enriched.tool_error_summary && errorContext.toolErrorSummary) {
    enriched.tool_error_summary = errorContext.toolErrorSummary;
  }

  return withReceiptHash(enriched);
}

function createRuntimeSystemEntrypoint(
  scriptDir,
  {
    lane,
    systemId,
    type,
    domain = 'runtime-systems',
    maxArgLen = DEFAULT_MAX_ARG_LEN,
    maxArgs = DEFAULT_MAX_ARGS,
    inheritStdio = true
  } = {}
) {
  const resolvedMaxArgLen = resolvePositiveIntegerOption(maxArgLen, DEFAULT_MAX_ARG_LEN, 16, 4096);
  const resolvedMaxArgs = resolvePositiveIntegerOption(maxArgs, DEFAULT_MAX_ARGS, 1, 256);
  const bridge = createOpsLaneBridge(scriptDir, lane, domain, {
    inheritStdio
  });

  function run(args = process.argv.slice(2)) {
    const argDetails = normalizeBridgeArgsDetailed(args, resolvedMaxArgLen, resolvedMaxArgs);
    const passthrough = argDetails.passthrough;
    const attemptSignature = computeAttemptSignature(systemId, passthrough);
    const mutationLikely = detectMutationLikely(passthrough);
    const out = bridge.run([`--system-id=${String(systemId || '')}`].concat(passthrough));
    if (out && out.stdout) process.stdout.write(out.stdout);
    if (out && out.stderr) process.stderr.write(out.stderr);

    const payloadError = resolveStructuredErrorDetails(out && out.payload ? out.payload.error : undefined);
    const stderrError = resolveStructuredErrorDetails(out && out.stderr);
    const runtimeError = resolveStructuredErrorDetails(out && out.error);
    const resolvedErrorText =
      payloadError.text || stderrError.text || runtimeError.text || normalizeErrorText(out && out.error);
    const retryAfterMs =
      extractRetryAfterMs(payloadError.record) ||
      extractRetryAfterMs(stderrError.record) ||
      extractRetryAfterMs(runtimeError.record);

    if (out && out.payload && !out.stdout) {
      const payload =
        out.payload && typeof out.payload === 'object' && !Array.isArray(out.payload)
          ? enrichObjectPayload(
              Object.assign(
                {
                  lane: bridge.lane,
                  system_id: String(systemId || '')
                },
                out.payload
              ),
              {
                type: String(type || lane || 'runtime_system_entrypoint'),
                status: normalizeExitCode(out),
                errorText: resolvedErrorText,
                errorCode: normalizeErrorText(out.payload.error_code),
                attemptSignature,
                mutationLikely,
                retryAfterMs,
                argPolicy: {
                  max_args: resolvedMaxArgs,
                  max_arg_len: resolvedMaxArgLen,
                  args_count: argDetails.raw_count,
                  sanitized_count: argDetails.sanitized_count,
                  dropped_count: argDetails.dropped_count,
                  truncated: argDetails.truncated
                }
              }
            )
          : out.payload;
      process.stdout.write(`${JSON.stringify(payload)}\n`);
    } else if (!out || (!out.stdout && !out.stderr)) {
      const status = normalizeExitCode(out);
      const fallbackErrorCode = 'bridge_no_output';
      const errorContext = buildErrorContext({
        type: String(type || lane || 'runtime_system_entrypoint'),
        status,
        error: resolvedErrorText,
        errorCode: fallbackErrorCode,
        attemptSignature,
        mutationLikely,
        retryAfterMs
      });
      process.stdout.write(
        `${JSON.stringify(
          withReceiptHash({
            ok: false,
            type: String(type || lane || 'runtime_system_entrypoint'),
            lane: bridge.lane,
            system_id: String(systemId || ''),
            attempt_signature: attemptSignature,
            mutation_likely: mutationLikely,
            arg_policy: {
              max_args: resolvedMaxArgs,
              max_arg_len: resolvedMaxArgLen,
              args_count: argDetails.raw_count,
              sanitized_count: argDetails.sanitized_count,
              dropped_count: argDetails.dropped_count,
              truncated: argDetails.truncated
            },
            error: fallbackErrorCode,
            error_code: errorContext.errorCode || fallbackErrorCode,
            error_class: errorContext.errorClass,
            retry: errorContext.retry,
            tool_error_summary: errorContext.toolErrorSummary,
            status
          })
        )}\n`
      );
    }
    return out;
  }

  function exitFromRun(args = process.argv.slice(2)) {
    const out = run(args);
    process.exit(normalizeExitCode(out));
  }

  return {
    lane: bridge.lane,
    systemId,
    run,
    exitFromRun,
    normalizeReceiptHash
  };
}

module.exports = {
  DEFAULT_MAX_ARGS,
  DEFAULT_MAX_ARG_LEN,
  sanitizeBridgeArg,
  createRuntimeSystemEntrypoint,
  normalizeReceiptHash
};
