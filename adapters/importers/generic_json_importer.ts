'use strict';
const crypto = require('node:crypto');
const path = require('path');

const ROOT = path.resolve(__dirname, '..', '..');
const PROXY_PATH = path.join(ROOT, 'client', 'lib', 'legacy_conduit_proxy.ts');
const { createDomainProxy } = require(PROXY_PATH);

type AnyObj = Record<string, any>;

const MAX_PAYLOAD_BYTES = 1024 * 1024;
const EMPTY_PAYLOAD = Object.freeze({
  entities: { agents: [], tasks: [], workflows: [], tools: [], records: [] },
  source_item_count: 0,
  mapped_item_count: 0,
  warnings: [],
});

const TRANSIENT_IMPORT_RE = /429|timeout|connect|reset|closed|temporar(?:y|ily)|unavailable|retry/i;
const NOT_FOUND_RE = /enoent|not\s+found|missing|no such/i;

function resolveSafeBaseDir(rootDir: string) {
  const resolved = path.resolve(rootDir);
  return resolved.endsWith(path.sep) ? resolved : `${resolved}${path.sep}`;
}

function isWithinDir(rootDir: string, targetPath: string) {
  const safeRoot = resolveSafeBaseDir(rootDir);
  const resolvedTarget = path.resolve(targetPath);
  return resolvedTarget === path.resolve(rootDir) || resolvedTarget.startsWith(safeRoot);
}

if (!isWithinDir(ROOT, PROXY_PATH)) {
  throw new Error('importer_proxy_path_outside_root');
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v)
    .replace(/[\u200B\u200C\u200D\u2060\uFEFF]/g, '')
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, maxLen);
}

function parseStrictInteger(value: unknown) {
  if (typeof value === 'number') return Number.isSafeInteger(value) ? value : undefined;
  if (typeof value !== 'string') return undefined;
  const normalized = value.trim();
  if (!normalized || !/^[+-]?\d+$/.test(normalized)) return undefined;
  const parsed = Number(normalized);
  return Number.isSafeInteger(parsed) ? parsed : undefined;
}

function parseStrictNonNegativeInteger(value: unknown) {
  const parsed = parseStrictInteger(value);
  return parsed !== undefined && parsed >= 0 ? parsed : undefined;
}

function stableStringify(value: unknown): string {
  if (value === null || typeof value !== 'object') return JSON.stringify(value);
  if (Array.isArray(value)) return `[${value.map((item) => stableStringify(item)).join(',')}]`;
  const keys = Object.keys(value as AnyObj).sort();
  return `{${keys.map((key) => `${JSON.stringify(key)}:${stableStringify((value as AnyObj)[key])}`).join(',')}}`;
}

function buildAttemptSignature(engine: string, encodedPayload: string) {
  return crypto
    .createHash('sha256')
    .update(stableStringify({ engine: cleanText(engine, 64), payload_base64: encodedPayload }))
    .digest('hex');
}

function extractJsonObjectCandidates(raw: string): string[] {
  const candidates: string[] = [];
  let depth = 0;
  let start = -1;
  let inString = false;
  let escaped = false;

  for (let i = 0; i < raw.length; i += 1) {
    const ch = raw[i] || '';
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
      if (depth === 0) start = i;
      depth += 1;
      continue;
    }
    if (ch === '}' && depth > 0) {
      depth -= 1;
      if (depth === 0 && start >= 0) {
        candidates.push(raw.slice(start, i + 1));
        start = -1;
      }
    }
  }

  return candidates;
}

function parseJsonRecordCandidates(raw: string): AnyObj[] {
  const parsedRecords: AnyObj[] = [];
  const trimmed = cleanText(raw, 8000);
  if (!trimmed) return parsedRecords;

  try {
    const parsed = JSON.parse(trimmed);
    if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
      parsedRecords.push(parsed as AnyObj);
      return parsedRecords;
    }
  } catch {
    // mixed output fallback below
  }

  for (const candidate of extractJsonObjectCandidates(trimmed)) {
    try {
      const parsed = JSON.parse(candidate);
      if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
        parsedRecords.push(parsed as AnyObj);
      }
    } catch {
      // ignore malformed fragments
    }
  }

  return parsedRecords;
}

function readNestedErrorMessage(record: AnyObj): string | undefined {
  if (!record || typeof record !== 'object') return undefined;
  if (record.error && typeof record.error === 'object' && !Array.isArray(record.error)) {
    const nested = readNestedErrorMessage(record.error as AnyObj);
    if (nested) return nested;
  }
  if (record.details && typeof record.details === 'object' && !Array.isArray(record.details)) {
    const nested = readNestedErrorMessage(record.details as AnyObj);
    if (nested) return nested;
  }
  if (typeof record.message === 'string' && record.message.trim()) return record.message.trim();
  if (typeof record.error === 'string' && record.error.trim()) return record.error.trim();
  if (typeof record.detail === 'string' && record.detail.trim()) return record.detail.trim();
  return undefined;
}

function formatErrorMessage(err: unknown): string {
  if (err instanceof Error) {
    let out = err.message || err.name || 'Error';
    let cause: unknown = (err as AnyObj).cause;
    const seen = new Set<unknown>([err]);
    while (cause && !seen.has(cause)) {
      seen.add(cause);
      if (cause instanceof Error) {
        if (cause.message) out += ` | ${cause.message}`;
        cause = (cause as AnyObj).cause;
      } else if (typeof cause === 'string') {
        out += ` | ${cause}`;
        break;
      } else {
        break;
      }
    }
    return cleanText(out, 1600);
  }
  if (typeof err === 'string') return cleanText(err, 1600);
  if (typeof err === 'number' || typeof err === 'boolean' || typeof err === 'bigint') return String(err);
  try {
    return cleanText(JSON.stringify(err), 1600);
  } catch {
    return cleanText(Object.prototype.toString.call(err), 1600);
  }
}

function unwrapCliErrorText(raw: string): string {
  const trimmed = cleanText(raw, 1600);
  if (!trimmed) return '';
  for (const parsed of parseJsonRecordCandidates(trimmed)) {
    const nested = readNestedErrorMessage(parsed);
    if (nested) return cleanText(nested, 1600);
  }
  return trimmed;
}

function inferRetryAfterMs(err: unknown): number | undefined {
  if (!err || typeof err !== 'object' || Array.isArray(err)) return undefined;
  const record = err as AnyObj;
  const direct = Number(record.retry_after_ms ?? record.retryAfterMs ?? record.retry_after ?? record.retryAfter);
  if (Number.isFinite(direct) && direct > 0) {
    return direct <= 300 ? Math.trunc(direct * 1000) : Math.trunc(direct);
  }
  for (const nested of [record.parameters, record.response, record.error, record.details]) {
    const nestedRetry = inferRetryAfterMs(nested);
    if (nestedRetry !== undefined) return nestedRetry;
  }
  return undefined;
}

function classifyImportError(errorText: string): 'transport_unavailable' | 'transient' | 'execution_error' | 'none' {
  const text = cleanText(errorText, 600).toLowerCase();
  if (!text) return 'none';
  if (NOT_FOUND_RE.test(text)) return 'transport_unavailable';
  if (TRANSIENT_IMPORT_RE.test(text)) return 'transient';
  return 'execution_error';
}

function retryHints(errorClass: string, retryAfterMs?: number) {
  if (errorClass === 'transient') {
    const minDelay = retryAfterMs !== undefined ? Math.max(250, retryAfterMs) : 400;
    return {
      recommended: true,
      strategy: 'bounded_backoff',
      lane: 'same_lane_retry',
      attempts: 2,
      min_delay_ms: minDelay,
      max_delay_ms: retryAfterMs !== undefined ? Math.max(minDelay, retryAfterMs) : 5000,
      jitter: retryAfterMs !== undefined ? 0 : 0.1,
      ...(retryAfterMs !== undefined ? { retry_after_ms: retryAfterMs } : {}),
    };
  }
  if (errorClass === 'transport_unavailable') {
    return {
      recommended: false,
      strategy: 'manual_recovery',
      lane: 'operator_fix',
    };
  }
  return {
    recommended: false,
    strategy: 'none',
    lane: 'none',
  };
}

function encodePayloadBase64(payload: unknown) {
  try {
    const serialized = JSON.stringify(payload == null ? {} : payload);
    const bytes = Buffer.byteLength(serialized, 'utf8');
    if (bytes > MAX_PAYLOAD_BYTES) {
      return {
        ok: false,
        error: `payload_too_large:${bytes}`,
        error_class: 'execution_error',
      };
    }
    return { ok: true, encoded: Buffer.from(serialized, 'utf8').toString('base64') };
  } catch (error) {
    const detail = formatErrorMessage(error);
    return {
      ok: false,
      error: unwrapCliErrorText(detail) || 'payload_encode_failed',
      error_class: 'execution_error',
    };
  }
}

function normalizeImportedPayload(payload: AnyObj, diagnostics: AnyObj = {}) {
  const entities = payload && typeof payload.entities === 'object' ? payload.entities : {};
  const sourceCount = parseStrictNonNegativeInteger(payload && payload.source_item_count) ?? 0;
  const mappedCount = parseStrictNonNegativeInteger(payload && payload.mapped_item_count) ?? 0;
  return {
    entities: {
      agents: Array.isArray(entities.agents) ? entities.agents : [],
      tasks: Array.isArray(entities.tasks) ? entities.tasks : [],
      workflows: Array.isArray(entities.workflows) ? entities.workflows : [],
      tools: Array.isArray(entities.tools) ? entities.tools : [],
      records: Array.isArray(entities.records) ? entities.records : []
    },
    source_item_count: sourceCount,
    mapped_item_count: mappedCount,
    warnings: Array.isArray(payload && payload.warnings)
      ? payload.warnings.map((v: unknown) => cleanText(v, 220)).filter(Boolean)
      : [],
    import_diagnostics: {
      attempt_signature: cleanText(diagnostics.attempt_signature, 120),
      engine: cleanText(diagnostics.engine, 80),
      route: 'conduit_importer',
      ...(diagnostics.error_class ? { error_class: diagnostics.error_class } : {}),
      ...(diagnostics.retry ? { retry: diagnostics.retry } : {}),
    },
  };
}

function createConduitImporter(
  engine: string,
  command: string,
  domainId: string,
) {
  const runDomain = createDomainProxy(__dirname, String(domainId || 'IMPORTER_GENERIC_JSON'), 'execution-yield-recovery');

  function runViaConduit(payloadBase64: string, attemptSignature: string) {
    const out = runDomain([String(command || 'importer-generic-json'), '--payload-base64=' + String(payloadBase64 || '')]);
    if (
      out &&
      out.ok === true &&
      out.payload &&
      typeof out.payload === 'object' &&
      out.payload.ok === true &&
      out.payload.payload &&
      typeof out.payload.payload === 'object'
    ) {
      return {
        ok: true,
        payload: out.payload.payload,
        attempt_signature: attemptSignature,
        engine: engine,
      };
    }

    const rawError =
      (out && out.error) ||
      (out && out.payload && typeof out.payload === 'object' && out.payload.error) ||
      (out && out.stderr) ||
      (out && out.stdout) ||
      'conduit_importer_unavailable';
    const error = unwrapCliErrorText(formatErrorMessage(rawError));
    const errorClass = classifyImportError(error);
    const parsedCandidates = parseJsonRecordCandidates(String(rawError || ''));
    const retryAfterMs = parsedCandidates
      .map((candidate) => inferRetryAfterMs(candidate))
      .find((value) => value !== undefined);

    return {
      ok: false,
      error: cleanText(error || 'conduit_importer_unavailable', 260),
      error_class: errorClass,
      retry: retryHints(errorClass, retryAfterMs),
      attempt_signature: attemptSignature,
      engine: engine,
    };
  }

  function importPayload(payload: unknown, context: AnyObj = {}) {
    void context;
    const encoded = encodePayloadBase64(payload);
    if (!encoded.ok) {
      const errorClass = cleanText(encoded.error_class || 'execution_error', 64);
      return {
        entities: { ...EMPTY_PAYLOAD.entities },
        source_item_count: 0,
        mapped_item_count: 0,
        warnings: ['payload_encode_failed:' + cleanText(encoded.error || 'unknown', 220)],
        import_diagnostics: {
          engine: engine,
          route: 'encode_failed',
          error_class: errorClass,
          retry: retryHints(errorClass),
        },
      };
    }

    const attemptSignature = buildAttemptSignature(engine, encoded.encoded);
    const result = runViaConduit(encoded.encoded, attemptSignature);
    if (result.ok && result.payload) {
      return normalizeImportedPayload(result.payload, {
        attempt_signature: result.attempt_signature,
        engine: result.engine,
      });
    }

    const err = cleanText(result.error || 'conduit_importer_unavailable', 220);
    return {
      entities: { agents: [], tasks: [], workflows: [], tools: [], records: [] },
      source_item_count: 0,
      mapped_item_count: 0,
      warnings: [`conduit_importer_unavailable:${err}`],
      import_diagnostics: {
        attempt_signature: cleanText(result.attempt_signature, 120),
        engine: cleanText(result.engine, 80),
        route: 'conduit_error',
        error_class: cleanText(result.error_class || 'execution_error', 64),
        retry: result.retry || retryHints('execution_error'),
      },
    };
  }

  return {
    engine: String(engine || 'generic_json'),
    importPayload
  };
}

const genericJsonImporter = createConduitImporter(
  'generic_json',
  'importer-generic-json',
  'IMPORTER_GENERIC_JSON',
);

module.exports = {
  ...genericJsonImporter,
  createConduitImporter
};
