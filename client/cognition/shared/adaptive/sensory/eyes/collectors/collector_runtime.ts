'use strict';

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const ts = require('typescript');
const http = require('http');
const https = require('https');
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
  if (existing && existing.__protheusTsHook === true) return;
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
  require.extensions['.ts'].__protheusTsHook = true;
}

installTsHook();

const { egressFetch, EgressGatewayError } = require('../../../../../../lib/egress_gateway.ts');
const { classifyCollectorError, httpStatusToCode, makeCollectorError, isRetryableCode } = require('./collector_errors.ts');

const ROOT = WORKSPACE_ROOT;
const EYES_STATE_DIR = process.env.EYES_STATE_DIR
  ? path.resolve(process.env.EYES_STATE_DIR)
  : path.join(ROOT, 'local', 'state', 'sensory', 'eyes');
const META_DIR = path.join(EYES_STATE_DIR, 'collector_meta');
const RATE_STATE_PATH = path.join(EYES_STATE_DIR, 'collector_rate_state.json');

function nowIso() {
  return new Date().toISOString();
}

function sha16(input) {
  return crypto.createHash('sha256').update(String(input || ''), 'utf8').digest('hex').slice(0, 16);
}

function clampInt(v, min, max, fallback) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(n)));
}

function cleanText(v, maxLen = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function ensureDir(dirPath) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath, fallback) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, value) {
  ensureDir(path.dirname(filePath));
  fs.writeFileSync(filePath, JSON.stringify(value, null, 2));
}

function appendJsonl(filePath, row) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`);
}

function htmlDecode(raw) {
  const s = String(raw || '');
  return s
    .replace(/<!\[CDATA\[([\s\S]*?)\]\]>/g, '$1')
    .replace(/&amp;/g, '&')
    .replace(/&lt;/g, '<')
    .replace(/&gt;/g, '>')
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&#x2F;/gi, '/');
}

function stripTags(raw) {
  return htmlDecode(String(raw || '').replace(/<[^>]*>/g, ' '));
}

function extractTagValue(block, tagName) {
  const re = new RegExp(`<${tagName}[^>]*>([\\s\\S]*?)<\\/${tagName}>`, 'i');
  const m = String(block || '').match(re);
  return m ? stripTags(m[1]) : '';
}

function extractTagAttr(block, tagName, attrName) {
  const re = new RegExp(`<${tagName}[^>]*\\b${attrName}="([^"]+)"[^>]*>`, 'i');
  const m = String(block || '').match(re);
  return m ? htmlDecode(m[1]) : '';
}

function extractEntries(xml) {
  const text = String(xml || '');
  const items = [];

  const rssMatches = text.match(/<item\b[\s\S]*?<\/item>/gi) || [];
  for (const block of rssMatches) {
    const title = extractTagValue(block, 'title');
    const link = extractTagValue(block, 'link') || extractTagValue(block, 'guid');
    const description = extractTagValue(block, 'description') || extractTagValue(block, 'content:encoded');
    const published = extractTagValue(block, 'pubDate') || extractTagValue(block, 'dc:date');
    if (!title && !link) continue;
    items.push({ title, link, description, published });
  }

  const atomMatches = text.match(/<entry\b[\s\S]*?<\/entry>/gi) || [];
  for (const block of atomMatches) {
    const title = extractTagValue(block, 'title');
    const link = extractTagAttr(block, 'link', 'href') || extractTagValue(block, 'id');
    const description = extractTagValue(block, 'summary') || extractTagValue(block, 'content');
    const published = extractTagValue(block, 'updated') || extractTagValue(block, 'published');
    if (!title && !link) continue;
    items.push({ title, link, description, published });
  }

  return items;
}

function directFetchText(url, options = {}) {
  return new Promise((resolve, reject) => {
    let parsed;
    try {
      parsed = new URL(String(url || ''));
    } catch (err) {
      reject(err);
      return;
    }
    const client = parsed.protocol === 'http:' ? http : https;
    const timeoutMs = clampInt(options.timeoutMs, 500, 120000, 15000);
    const req = client.request(parsed, {
      method: 'GET',
      headers: options.headers && typeof options.headers === 'object' ? options.headers : {}
    }, (res) => {
      const chunks = [];
      res.on('data', (chunk) => chunks.push(Buffer.from(chunk)));
      res.on('end', () => {
        const body = Buffer.concat(chunks).toString('utf8');
        resolve({
          status: Number(res.statusCode || 0),
          text: body,
          bytes: Buffer.byteLength(body, 'utf8')
        });
      });
    });
    req.on('error', reject);
    req.setTimeout(timeoutMs, () => req.destroy(new Error(`direct_fetch_timeout:${timeoutMs}`)));
    req.end();
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, Math.max(0, Number(ms) || 0)));
}

function loadRateState() {
  return readJson(RATE_STATE_PATH, { schema_id: 'collector_rate_state_v1', collectors: {} });
}

function saveRateState(state) {
  writeJson(RATE_STATE_PATH, state);
}

function collectorRateRow(state, collectorId) {
  if (!state.collectors || typeof state.collectors !== 'object') state.collectors = {};
  if (!state.collectors[collectorId] || typeof state.collectors[collectorId] !== 'object') {
    state.collectors[collectorId] = {
      last_attempt_ms: 0,
      last_success_ms: 0,
      failure_streak: 0,
      next_allowed_ms: 0,
      circuit_open_until_ms: 0,
      last_error_code: null
    };
  }
  return state.collectors[collectorId];
}

function loadCollectorMeta(collectorId) {
  const p = path.join(META_DIR, `${collectorId}.json`);
  const base = {
    collector_id: collectorId,
    last_run: null,
    last_success: null,
    seen_ids: []
  };
  const raw = readJson(p, base);
  if (!raw || typeof raw !== 'object') return { path: p, meta: base };
  return {
    path: p,
    meta: {
      collector_id: collectorId,
      last_run: typeof raw.last_run === 'string' ? raw.last_run : null,
      last_success: typeof raw.last_success === 'string' ? raw.last_success : null,
      seen_ids: Array.isArray(raw.seen_ids) ? raw.seen_ids.slice(-2000) : []
    }
  };
}

function saveCollectorMeta(metaPath, meta) {
  writeJson(metaPath, meta);
}

async function fetchTextWithAdaptiveControls(collectorId, url, options = {}) {
  const scope = cleanText(options.scope || `sensory.collector.${collectorId}`, 120);
  const caller = cleanText(options.caller || `adaptive/sensory/eyes/collectors/${collectorId}`, 200);
  const timeoutMs = clampInt(options.timeoutMs, 1000, 120000, 15000);
  const attempts = clampInt(options.attempts, 1, 5, 3);
  const minIntervalMs = clampInt(options.minIntervalMs, 50, 30000, Number(process.env.EYES_COLLECTOR_MIN_INTERVAL_MS || 300));
  const baseBackoffMs = clampInt(options.baseBackoffMs, 50, 30000, Number(process.env.EYES_COLLECTOR_BACKOFF_BASE_MS || 300));
  const maxBackoffMs = clampInt(options.maxBackoffMs, 200, 120000, Number(process.env.EYES_COLLECTOR_BACKOFF_MAX_MS || 8000));
  const circuitOpenMs = clampInt(options.circuitOpenMs, 500, 300000, Number(process.env.EYES_COLLECTOR_CIRCUIT_MS || 30000));
  const circuitAfterFailures = clampInt(options.circuitAfterFailures, 1, 10, Number(process.env.EYES_COLLECTOR_CIRCUIT_AFTER || 3));
  const headers = options.headers && typeof options.headers === 'object' ? options.headers : {
    'User-Agent': 'Infring-Eyes/1.0',
    'Accept': 'application/rss+xml,application/atom+xml,application/json,text/xml,text/html;q=0.9,*/*;q=0.8'
  };

  let lastErr = null;
  for (let attempt = 1; attempt <= attempts; attempt += 1) {
    const state = loadRateState();
    const row = collectorRateRow(state, collectorId);
    const now = Date.now();

    if (row.circuit_open_until_ms > now) {
      throw makeCollectorError('rate_limited', `collector_circuit_open:${collectorId}`, {
        retry_after_ms: row.circuit_open_until_ms - now,
        collector: collectorId
      });
    }

    const readyAt = Math.max(Number(row.next_allowed_ms || 0), Number(row.last_attempt_ms || 0) + minIntervalMs);
    const waitMs = Math.max(0, readyAt - now);
    if (waitMs > 0) {
      await sleep(waitMs);
    }

    row.last_attempt_ms = Date.now();
    saveRateState(state);

    try {
      const host = new URL(url).hostname;
      let status = 0;
      let text = '';
      let bytes = 0;
      try {
        const res = await egressFetch(url, {
          method: 'GET',
          headers
        }, {
          scope,
          caller,
          runtime_allowlist: [host],
          timeout_ms: timeoutMs,
          meta: {
            collector: collectorId,
            attempt
          }
        });
        status = Number(res.status || 0);
        text = String(await res.text());
        bytes = Buffer.byteLength(text, 'utf8');
      } catch (err) {
        const allowDirectFallback = String(process.env.EYES_COLLECTOR_ALLOW_DIRECT_FETCH_FALLBACK || '0') === '1';
        if (!(err instanceof EgressGatewayError) || !allowDirectFallback) {
          throw err;
        }
        const direct = await directFetchText(url, { timeoutMs, headers });
        status = Number(direct.status || 0);
        text = String(direct.text || '');
        bytes = Number(direct.bytes || 0);
      }
      if (status >= 400) {
        throw makeCollectorError(
          httpStatusToCode(status),
          `HTTP ${status} for ${url}`,
          { http_status: Number(status), url }
        );
      }

      const successState = loadRateState();
      const successRow = collectorRateRow(successState, collectorId);
      successRow.last_success_ms = Date.now();
      successRow.failure_streak = 0;
      successRow.next_allowed_ms = Date.now() + minIntervalMs;
      successRow.circuit_open_until_ms = 0;
      successRow.last_error_code = null;
      saveRateState(successState);

      return { text, bytes, status: Number(status || 200), attempt };
    } catch (err) {
      const normalized = err instanceof EgressGatewayError
        ? makeCollectorError(
            'env_blocked',
            `egress_denied:${String(err.details && err.details.code || 'policy')} for ${url}`.slice(0, 220),
            { url }
          )
        : err;
      const classified = classifyCollectorError(normalized);
      lastErr = makeCollectorError(classified.code, classified.message, {
        http_status: classified.http_status,
        url,
        attempt
      });

      const failState = loadRateState();
      const failRow = collectorRateRow(failState, collectorId);
      if (isRetryableCode(classified.code)) {
        failRow.failure_streak = Number(failRow.failure_streak || 0) + 1;
        const exp = Math.max(0, failRow.failure_streak - 1);
        const backoffMs = Math.min(maxBackoffMs, baseBackoffMs * (2 ** exp));
        failRow.next_allowed_ms = Date.now() + backoffMs;
        if (failRow.failure_streak >= circuitAfterFailures) {
          failRow.circuit_open_until_ms = Date.now() + circuitOpenMs;
        }
      } else {
        failRow.failure_streak = Number(failRow.failure_streak || 0) + 1;
        failRow.next_allowed_ms = Date.now() + Math.min(maxBackoffMs, baseBackoffMs);
      }
      failRow.last_error_code = classified.code;
      saveRateState(failState);

      if (!classified.retryable || attempt >= attempts) {
        throw lastErr;
      }
    }
  }

  throw lastErr || makeCollectorError('collector_error', `fetch_failed:${collectorId}`, { url });
}

function mapEntriesToItems(entries, config, seenIds, bytesPerEntry) {
  const out = [];
  const topics = Array.isArray(config.topics) ? config.topics.slice(0, 8) : [];
  const signalRe = config.signalRegex instanceof RegExp
    ? config.signalRegex
    : (String(config.signalRegex || '') ? new RegExp(String(config.signalRegex), 'i') : null);
  const maxItems = clampInt(config.maxItems, 1, 200, 20);

  for (const entry of Array.isArray(entries) ? entries : []) {
    if (out.length >= maxItems) break;
    const title = cleanText(entry && entry.title, 220);
    const url = cleanText(entry && entry.link, 500);
    const description = cleanText(entry && entry.description, 420);
    const publishedAt = cleanText(entry && entry.published, 120);
    if (!title || !url) continue;
    const id = sha16(`${config.collectorId}|${url}|${title}`);
    if (seenIds.has(id)) continue;
    seenIds.add(id);

    const signal = signalRe ? signalRe.test(`${title} ${description}`) : false;
    out.push({
      id,
      collected_at: nowIso(),
      url,
      title,
      description,
      published_at: publishedAt || null,
      source: config.collectorId,
      signal,
      signal_type: signal ? 'high_signal' : 'feed_item',
      topics,
      tags: [config.collectorId, signal ? 'signal' : 'watch'],
      bytes: bytesPerEntry
    });
  }

  return out;
}

async function runFeedCollector(config = {}) {
  const collectorId = cleanText(config.collectorId || 'feed_collector', 80).toLowerCase();
  const scope = cleanText(config.scope || `sensory.collector.${collectorId}`, 120);
  const caller = cleanText(config.caller || `adaptive/sensory/eyes/collectors/${collectorId}`, 220);
  const feedCandidates = Array.isArray(config.feedCandidates)
    ? config.feedCandidates.map((v) => cleanText(v, 600)).filter(Boolean)
    : [];
  const minHours = Number.isFinite(Number(config.minHours)) ? Number(config.minHours) : 4;
  const force = config.force === true;

  const { path: metaPath, meta } = loadCollectorMeta(collectorId);
  const lastRunMs = meta.last_run ? new Date(meta.last_run).getTime() : 0;
  const hoursSince = lastRunMs > 0 ? (Date.now() - lastRunMs) / 3600000 : Number.POSITIVE_INFINITY;
  if (!force && Number.isFinite(hoursSince) && hoursSince < minHours) {
    return {
      ok: true,
      success: true,
      eye: collectorId,
      skipped: true,
      reason: 'cadence',
      hours_since_last: Number(hoursSince.toFixed(2)),
      min_hours: minHours,
      items: []
    };
  }

  const seenIds = new Set(Array.isArray(meta.seen_ids) ? meta.seen_ids : []);
  const startedAt = Date.now();
  const attempts = clampInt(config.attempts, 1, 5, 3);
  let totalBytes = 0;
  let requests = 0;
  let entries = [];
  let finalError = null;

  for (const feedUrl of feedCandidates) {
    try {
      const fetched = await fetchTextWithAdaptiveControls(collectorId, feedUrl, {
        scope,
        caller,
        timeoutMs: config.timeoutMs,
        attempts,
        minIntervalMs: config.minIntervalMs,
        baseBackoffMs: config.baseBackoffMs,
        maxBackoffMs: config.maxBackoffMs,
        circuitOpenMs: config.circuitOpenMs,
        circuitAfterFailures: config.circuitAfterFailures,
        headers: config.headers
      });
      requests += 1;
      totalBytes += Number(fetched.bytes || 0);
      const parsed = extractEntries(fetched.text);
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
    const bytesPerEntry = Math.max(64, Math.floor(totalBytes / Math.max(entries.length, 1)));
    items = mapEntriesToItems(entries, {
      collectorId,
      maxItems: config.maxItems,
      topics: config.topics,
      signalRegex: config.signalRegex
    }, seenIds, bytesPerEntry);
  }

  if (items.length === 0) {
    const cacheEnvelope = readJson(path.join(META_DIR, `${collectorId}.cache.json`), { items: [] });
    const cached = Array.isArray(cacheEnvelope.items) ? cacheEnvelope.items : [];
    if (cached.length > 0) {
      return {
        ok: true,
        success: true,
        eye: collectorId,
        cache_hit: true,
        items: cached.slice(0, clampInt(config.maxItems, 1, 200, 20)),
        bytes: cached.reduce((sum, row) => sum + Number((row && row.bytes) || 0), 0),
        requests,
        duration_ms: Date.now() - startedAt,
        degraded: Boolean(finalError),
        error: finalError ? cleanText(finalError.code || finalError.message || 'collector_error', 120) : null,
        cadence_hours: minHours
      };
    }
  }

  meta.last_run = nowIso();
  if (items.length > 0) {
    meta.last_success = meta.last_run;
    meta.seen_ids = Array.from(seenIds).slice(-2000);
    writeJson(path.join(META_DIR, `${collectorId}.cache.json`), { items });
  }
  saveCollectorMeta(metaPath, meta);

  if (items.length === 0 && finalError) {
    const c = classifyCollectorError(finalError);
    return {
      ok: false,
      success: false,
      eye: collectorId,
      items: [],
      bytes: totalBytes,
      requests,
      duration_ms: Date.now() - startedAt,
      error: c.code || 'collector_error',
      http_status: c.http_status || null,
      cadence_hours: minHours
    };
  }

  return {
    ok: true,
    success: true,
    eye: collectorId,
    items,
    bytes: totalBytes,
    requests,
    duration_ms: Date.now() - startedAt,
    cadence_hours: minHours,
    sample: items[0] && items[0].title ? cleanText(items[0].title, 120) : null
  };
}

async function runJsonCollector(config = {}) {
  const collectorId = cleanText(config.collectorId || 'json_collector', 80).toLowerCase();
  const scope = cleanText(config.scope || `sensory.collector.${collectorId}`, 120);
  const caller = cleanText(config.caller || `adaptive/sensory/eyes/collectors/${collectorId}`, 220);
  const url = cleanText(config.url, 600);
  const minHours = Number.isFinite(Number(config.minHours)) ? Number(config.minHours) : 4;
  const maxItems = clampInt(config.maxItems, 1, 200, 20);
  const force = config.force === true;

  const { path: metaPath, meta } = loadCollectorMeta(collectorId);
  const lastRunMs = meta.last_run ? new Date(meta.last_run).getTime() : 0;
  const hoursSince = lastRunMs > 0 ? (Date.now() - lastRunMs) / 3600000 : Number.POSITIVE_INFINITY;
  if (!force && Number.isFinite(hoursSince) && hoursSince < minHours) {
    return {
      ok: true,
      success: true,
      eye: collectorId,
      skipped: true,
      reason: 'cadence',
      hours_since_last: Number(hoursSince.toFixed(2)),
      min_hours: minHours,
      items: []
    };
  }

  try {
    const fetched = await fetchTextWithAdaptiveControls(collectorId, url, {
      scope,
      caller,
      timeoutMs: config.timeoutMs,
      attempts: config.attempts,
      minIntervalMs: config.minIntervalMs,
      baseBackoffMs: config.baseBackoffMs,
      maxBackoffMs: config.maxBackoffMs,
      circuitOpenMs: config.circuitOpenMs,
      circuitAfterFailures: config.circuitAfterFailures,
      headers: config.headers
    });
    let payload = null;
    try {
      payload = JSON.parse(String(fetched.text || ''));
    } catch {
      payload = null;
    }
    if (!payload) {
      throw makeCollectorError('parse_failed', `invalid_json:${collectorId}`, { url });
    }

    const extractor = typeof config.extractor === 'function'
      ? config.extractor
      : (() => []);
    const rows = Array.isArray(extractor(payload)) ? extractor(payload) : [];
    const seenIds = new Set(Array.isArray(meta.seen_ids) ? meta.seen_ids : []);
    const items = [];

    for (const row of rows) {
      if (items.length >= maxItems) break;
      const title = cleanText(row && row.title, 220);
      const link = cleanText(row && row.url, 500);
      if (!title || !link) continue;
      const id = sha16(`${collectorId}|${link}|${title}`);
      if (seenIds.has(id)) continue;
      seenIds.add(id);
      items.push({
        id,
        collected_at: nowIso(),
        url: link,
        title,
        description: cleanText(row && row.description, 420),
        source: collectorId,
        signal: row && row.signal === true,
        signal_type: cleanText(row && row.signal_type, 80) || ((row && row.signal === true) ? 'high_signal' : 'feed_item'),
        topics: Array.isArray(row && row.topics) ? row.topics.slice(0, 8) : (Array.isArray(config.topics) ? config.topics.slice(0, 8) : []),
        tags: Array.isArray(row && row.tags) ? row.tags.slice(0, 6) : [collectorId],
        bytes: Math.max(64, Number(row && row.bytes) || 0),
        published_at: cleanText(row && row.published_at, 120) || null
      });
    }

    meta.last_run = nowIso();
    if (items.length > 0) {
      meta.last_success = meta.last_run;
      meta.seen_ids = Array.from(seenIds).slice(-2000);
      writeJson(path.join(META_DIR, `${collectorId}.cache.json`), { items });
    }
    saveCollectorMeta(metaPath, meta);

    return {
      ok: true,
      success: true,
      eye: collectorId,
      items,
      bytes: Number(fetched.bytes || 0),
      requests: 1,
      duration_ms: 0,
      cadence_hours: minHours,
      sample: items[0] && items[0].title ? cleanText(items[0].title, 120) : null
    };
  } catch (err) {
    const cacheEnvelope = readJson(path.join(META_DIR, `${collectorId}.cache.json`), { items: [] });
    const cached = Array.isArray(cacheEnvelope.items) ? cacheEnvelope.items : [];
    if (cached.length > 0) {
      return {
        ok: true,
        success: true,
        eye: collectorId,
        cache_hit: true,
        degraded: true,
        error: cleanText(err && (err.code || err.message) || 'collector_error', 120),
        items: cached.slice(0, maxItems),
        bytes: cached.reduce((sum, row) => sum + Number((row && row.bytes) || 0), 0),
        requests: 1,
        duration_ms: 0,
        cadence_hours: minHours
      };
    }
    const c = classifyCollectorError(err);
    return {
      ok: false,
      success: false,
      eye: collectorId,
      items: [],
      bytes: 0,
      requests: 1,
      duration_ms: 0,
      error: c.code || 'collector_error',
      http_status: c.http_status || null,
      cadence_hours: minHours
    };
  }
}

module.exports = {
  nowIso,
  sha16,
  cleanText,
  stripTags,
  extractEntries,
  runFeedCollector,
  runJsonCollector,
  fetchTextWithAdaptiveControls,
  appendJsonl,
  readJson,
  writeJson,
  ensureDir,
  EYES_STATE_DIR
};
