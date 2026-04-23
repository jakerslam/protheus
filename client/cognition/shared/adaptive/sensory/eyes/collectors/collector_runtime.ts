'use strict';

const fs = require('fs');
const path = require('path');
const ts = require('typescript');
const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.ts');
function resolveWorkspaceRoot(startDir = __dirname) {
  let dir = path.resolve(startDir);
  while (true) {
    const marker = path.join(dir, 'core', 'layer0', 'ops', 'Cargo.toml');
    if (fs.existsSync(marker)) return dir;
    const parent = path.dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return path.resolve(startDir, '../../../../../../..');
}
const WORKSPACE_ROOT = resolveWorkspaceRoot();

process.env.EGRESS_GATEWAY_POLICY_PATH = path.join(
  WORKSPACE_ROOT,
  'client',
  'runtime',
  'config',
  'egress_gateway_policy.json'
);

function installTsHook() {
  const existing = require.extensions['.ts'];
  if (existing && existing.__infringTsHook === true) return;
  require.extensions['.ts'] = function transpileTs(module, filename) {
    const src = fs.readFileSync(filename, 'utf8');
    const out = ts.transpileModule(src, {
      compilerOptions: {
        module: ts.ModuleKind.CommonJS,
        target: ts.ScriptTarget.ES2022,
        moduleResolution: ts.ModuleResolutionKind.NodeJs,
        esModuleInterop: true,
        allowSyntheticDefaultImports: true
      },
      fileName: filename,
      reportDiagnostics: false
    }).outputText;
    module._compile(out, filename);
  };
  require.extensions['.ts'].__infringTsHook = true;
}

installTsHook();

const { makeCollectorError } = require('./collector_errors.ts');

const ROOT = WORKSPACE_ROOT;
const EYES_STATE_DIR = process.env.EYES_STATE_DIR
  ? path.resolve(process.env.EYES_STATE_DIR)
  : path.join(ROOT, 'local', 'state', 'sensory', 'eyes');
const RATE_STATE_PATH = path.join(EYES_STATE_DIR, 'collector_rate_state.json');

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';
const collectorRuntimeBridge = createOpsLaneBridge(
  __dirname,
  'collector_runtime',
  'collector-runtime-kernel',
  { preferLocalCore: true }
);
const collectorContentBridge = createOpsLaneBridge(
  __dirname,
  'collector_content',
  'collector-content-kernel',
  { preferLocalCore: true }
);

function nowIso() {
  return new Date().toISOString();
}

function clampInt(v, min, max, fallback) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(n)));
}

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function invokeCollectorRuntimeKernel(command, payload = {}) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = collectorRuntimeBridge.run([
    command,
    `--payload-base64=${encoded}`
  ]);
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      out.stderr || (out.payload && out.payload.error) || out.stdout || '',
      220
    );
    throw makeCollectorError(
      'collector_error',
      detail || `collector_runtime_kernel_${command}_failed:status=${status}`
    );
  }
  const payloadOut = out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || payloadOut.ok !== true) {
    throw makeCollectorError(
      'collector_error',
      cleanText(`collector_runtime_kernel_${command}_invalid_payload`, 220)
    );
  }
  return payloadOut;
}

function invokeCollectorContentKernel(command, payload = {}) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = collectorContentBridge.run([
    command,
    `--payload-base64=${encoded}`
  ]);
  const status = Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      out.stderr || (out.payload && out.payload.error) || out.stdout || '',
      220
    );
    throw makeCollectorError(
      'collector_error',
      detail || `collector_content_kernel_${command}_failed:status=${status}`
    );
  }
  const payloadOut = out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || payloadOut.ok !== true) {
    throw makeCollectorError(
      'collector_error',
      cleanText(`collector_content_kernel_${command}_invalid_payload`, 220)
    );
  }
  return payloadOut;
}

function classifyErrorViaKernel(err) {
  const out = invokeCollectorRuntimeKernel('classify-error', {
    code: err && err.code ? String(err.code) : '',
    message: err && err.message ? String(err.message) : String(err || 'collector_error'),
    http_status: err && Number.isFinite(Number(err.http_status || err.status))
      ? Number(err.http_status || err.status)
      : null,
  });
  return out && typeof out === 'object' ? out : {
    code: 'collector_error',
    message: 'collector_error',
    http_status: null,
    transport: false,
    retryable: true,
  };
}

async function fetchTextWithAdaptiveControls(collectorId, url, options = {}) {
  const scope = cleanText(options.scope, 120) || `sensory.collector.${collectorId}`;
  const caller = cleanText(options.caller, 200) || `adaptive/sensory/eyes/collectors/${collectorId}`;
  const timeoutMs = clampInt(options.timeout_ms ?? options.timeoutMs, 1000, 120000, 15000);
  const attempts = clampInt(options.attempts, 1, 5, 3);
  const minIntervalMs = clampInt(options.min_interval_ms ?? options.minIntervalMs, 50, 30000, 300);
  const baseBackoffMs = clampInt(options.base_backoff_ms ?? options.baseBackoffMs, 50, 30000, 300);
  const maxBackoffMs = clampInt(options.max_backoff_ms ?? options.maxBackoffMs, 200, 120000, 8000);
  const circuitOpenMs = clampInt(options.circuit_open_ms ?? options.circuitOpenMs, 500, 300000, 30000);
  const circuitAfterFailures = clampInt(options.circuit_after_failures ?? options.circuitAfterFailures, 1, 10, 3);
  const allowDirectFallback = options.allow_direct_fetch_fallback === true
    || options.allowDirectFetchFallback === true
    || String(process.env.EYES_COLLECTOR_ALLOW_DIRECT_FETCH_FALLBACK || '0') === '1';
  const headers = options.headers && typeof options.headers === 'object' ? options.headers : {
    'User-Agent': 'Infring-Eyes/1.0',
    'Accept': 'application/rss+xml,application/atom+xml,application/json,text/xml,text/html;q=0.9,*/*;q=0.8'
  };

  try {
    const fetched = invokeCollectorRuntimeKernel('fetch-text', {
      collector_id: collectorId,
      url,
      scope,
      caller,
      timeout_ms: timeoutMs,
      attempts,
      min_interval_ms: minIntervalMs,
      base_backoff_ms: baseBackoffMs,
      max_backoff_ms: maxBackoffMs,
      circuit_open_ms: circuitOpenMs,
      circuit_after_failures: circuitAfterFailures,
      allow_direct_fetch_fallback: allowDirectFallback === true,
      headers,
      rate_state_path: RATE_STATE_PATH,
    });
    return {
      text: String(fetched && fetched.text || ''),
      bytes: Number(fetched && fetched.bytes || 0),
      status: Number(fetched && fetched.status || 200),
      attempt: Number(fetched && fetched.attempt || 1),
    };
  } catch (err) {
    const classified = classifyErrorViaKernel(err);
    throw makeCollectorError(
      classified && classified.code ? String(classified.code) : 'collector_error',
      classified && classified.message ? String(classified.message) : `fetch_failed:${collectorId}`,
      {
        http_status: classified ? classified.http_status : null,
        url
      }
    );
  }
}

async function runFeedCollector(config = {}) {
  const begin = invokeCollectorRuntimeKernel('begin-collection', {
    collector_id: config.collectorId || 'feed_collector',
    scope: config.scope,
    caller: config.caller,
    timeout_ms: config.timeoutMs,
    attempts: config.attempts,
    min_interval_ms: config.minIntervalMs,
    base_backoff_ms: config.baseBackoffMs,
    max_backoff_ms: config.maxBackoffMs,
    circuit_open_ms: config.circuitOpenMs,
    circuit_after_failures: config.circuitAfterFailures,
    min_hours: config.minHours,
    max_items: config.maxItems,
    force: config.force === true,
    allow_direct_fetch_fallback: config.allowDirectFetchFallback,
    feed_candidates: config.feedCandidates,
    eyes_state_dir: EYES_STATE_DIR
  });
  if (begin && begin.skipped === true) {
    return begin;
  }
  const resolved = begin && begin.controls && typeof begin.controls === 'object'
    ? begin.controls
    : {};
  const collectorId = cleanText((begin && begin.eye) || resolved.collector_id, 80) || 'feed_collector';
  const scope = cleanText(resolved.scope, 120) || `sensory.collector.${collectorId}`;
  const caller = cleanText(resolved.caller, 220) || `adaptive/sensory/eyes/collectors/${collectorId}`;
  const feedCandidates = Array.isArray(resolved.feed_candidates)
    ? resolved.feed_candidates.map((v) => cleanText(v, 600)).filter(Boolean)
    : [];
  const minHours = Number.isFinite(Number(begin && begin.min_hours)) ? Number(begin.min_hours) : 4;
  const maxItems = clampInt(begin && begin.max_items, 1, 200, 20);
  const meta = begin && begin.meta && typeof begin.meta === 'object' ? begin.meta : {};
  const seenIds = Array.isArray(meta.seen_ids) ? meta.seen_ids : [];
  const startedAt = Date.now();
  const attempts = clampInt(resolved.attempts, 1, 5, 3);
  let totalBytes = 0;
  let requests = 0;
  let entries = [];
  let finalError = null;

  for (const feedUrl of feedCandidates) {
    try {
      const fetched = await fetchTextWithAdaptiveControls(collectorId, feedUrl, {
        scope,
        caller,
        timeout_ms: resolved.timeout_ms,
        attempts,
        min_interval_ms: resolved.min_interval_ms,
        base_backoff_ms: resolved.base_backoff_ms,
        max_backoff_ms: resolved.max_backoff_ms,
        circuit_open_ms: resolved.circuit_open_ms,
        circuit_after_failures: resolved.circuit_after_failures,
        allow_direct_fetch_fallback: resolved.allow_direct_fetch_fallback === true,
        headers: config.headers
      });
      requests += 1;
      totalBytes += Number(fetched.bytes || 0);
      const parsedEnvelope = invokeCollectorContentKernel('extract-entries', {
        xml: fetched.text
      });
      const parsed = Array.isArray(parsedEnvelope.entries) ? parsedEnvelope.entries : [];
      if (parsed.length > 0) {
        entries = parsed;
        break;
      }
    } catch (err) {
      finalError = err;
    }
  }

  let items = [];
  if (entries.length > 0) {
    try {
      const bytesPerEntry = Math.max(64, Math.floor(totalBytes / Math.max(entries.length, 1)));
      const mapped = invokeCollectorContentKernel('map-feed-items', {
        collector_id: collectorId,
        entries,
        seen_ids: seenIds,
        maxItems,
        topics: config.topics,
        signal_regex: config.signalRegex instanceof RegExp
          ? config.signalRegex.source
          : String(config.signalRegex || ''),
        bytes_per_entry: bytesPerEntry
      });
      items = Array.isArray(mapped.items) ? mapped.items : [];
      meta.seen_ids = Array.isArray(mapped.seen_ids) ? mapped.seen_ids.slice(-2000) : seenIds;
    } catch (err) {
      finalError = err;
    }
  }
  const classified = finalError ? classifyErrorViaKernel(finalError) : null;
  return invokeCollectorRuntimeKernel('finalize-run', {
    collector_id: collectorId,
    eyes_state_dir: EYES_STATE_DIR,
    meta,
    items,
    bytes: totalBytes,
    requests,
    duration_ms: Date.now() - startedAt,
    min_hours: minHours,
    max_items: maxItems,
    use_cache_when_empty: true,
    fetch_error_code: classified ? cleanText(classified.code || 'collector_error', 80) : null,
    http_status: classified ? Number(classified.http_status || 0) || null : null,
  });
}

async function runJsonCollector(config = {}) {
  const begin = invokeCollectorRuntimeKernel('begin-collection', {
    collector_id: config.collectorId || 'json_collector',
    scope: config.scope,
    caller: config.caller,
    timeout_ms: config.timeoutMs,
    attempts: config.attempts,
    min_interval_ms: config.minIntervalMs,
    base_backoff_ms: config.baseBackoffMs,
    max_backoff_ms: config.maxBackoffMs,
    circuit_open_ms: config.circuitOpenMs,
    circuit_after_failures: config.circuitAfterFailures,
    min_hours: config.minHours,
    max_items: config.maxItems,
    force: config.force === true,
    allow_direct_fetch_fallback: config.allowDirectFetchFallback,
    url: config.url,
    eyes_state_dir: EYES_STATE_DIR
  });
  if (begin && begin.skipped === true) {
    return begin;
  }
  const resolved = begin && begin.controls && typeof begin.controls === 'object'
    ? begin.controls
    : {};
  const collectorId = cleanText((begin && begin.eye) || resolved.collector_id, 80) || 'json_collector';
  const scope = cleanText(resolved.scope, 120) || `sensory.collector.${collectorId}`;
  const caller = cleanText(resolved.caller, 220) || `adaptive/sensory/eyes/collectors/${collectorId}`;
  const url = cleanText(resolved.url, 600);
  const minHours = Number.isFinite(Number(begin && begin.min_hours)) ? Number(begin.min_hours) : 4;
  const maxItems = clampInt(begin && begin.max_items, 1, 200, 20);
  const meta = begin && begin.meta && typeof begin.meta === 'object' ? begin.meta : {};
  const seenIds = Array.isArray(meta.seen_ids) ? meta.seen_ids : [];
  const startedAt = Date.now();
  let fetchedBytes = 0;
  let items = [];
  let finalError = null;
  try {
    const fetched = await fetchTextWithAdaptiveControls(collectorId, url, {
      scope,
      caller,
      timeout_ms: resolved.timeout_ms,
      attempts: resolved.attempts,
      min_interval_ms: resolved.min_interval_ms,
      base_backoff_ms: resolved.base_backoff_ms,
      max_backoff_ms: resolved.max_backoff_ms,
      circuit_open_ms: resolved.circuit_open_ms,
      circuit_after_failures: resolved.circuit_after_failures,
      allow_direct_fetch_fallback: resolved.allow_direct_fetch_fallback === true,
      headers: config.headers
    });
    fetchedBytes = Number(fetched.bytes || 0);
    let payload = null;
    try {
      payload = JSON.parse(String(fetched.text || ''));
    } catch {
      payload = null;
    }
    if (!payload) {
      throw makeCollectorError('parse_failed', `invalid_json:${collectorId}`, { url });
    }

    const extracted = invokeCollectorContentKernel('extract-json-rows', {
      collector_id: collectorId,
      payload,
      topics: config.topics
    });
    const rows = Array.isArray(extracted && extracted.rows) ? extracted.rows : [];
    const mapped = invokeCollectorContentKernel('map-json-items', {
      collector_id: collectorId,
      rows,
      seen_ids: seenIds,
      max_items: maxItems,
      topics: config.topics
    });
    items = Array.isArray(mapped.items) ? mapped.items : [];
    meta.seen_ids = Array.isArray(mapped.seen_ids) ? mapped.seen_ids.slice(-2000) : [];
  } catch (err) {
    finalError = err;
  }

  const classified = finalError ? classifyErrorViaKernel(finalError) : null;
  return invokeCollectorRuntimeKernel('finalize-run', {
    collector_id: collectorId,
    eyes_state_dir: EYES_STATE_DIR,
    meta,
    items,
    bytes: fetchedBytes,
    requests: 1,
    duration_ms: Date.now() - startedAt,
    min_hours: minHours,
    max_items: maxItems,
    use_cache_when_empty: false,
    fetch_error_code: classified ? cleanText(classified.code || 'collector_error', 80) : null,
    http_status: classified ? Number(classified.http_status || 0) || null : null,
  });
}

module.exports = {
  nowIso,
  cleanText,
  runFeedCollector,
  runJsonCollector,
  fetchTextWithAdaptiveControls,
  EYES_STATE_DIR
};
