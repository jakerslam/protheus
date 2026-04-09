/**
 * adaptive/sensory/eyes/collectors/moltbook_hot.ts
 *
 * Thin wrapper over Rust-authoritative moltbook-hot collector kernel.
 * Client side keeps only secret-handle issuance + bridge wiring.
 */

const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.ts');
const { issueSecretHandle, loadSecretById } = require('../../../../../../runtime/lib/secret_broker.ts');
const { makeCollectorError } = require('./collector_errors.ts');
const { loadCollectorCache, saveCollectorCache } = require('./cache_store.ts');

process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';

const moltbookHotBridge = createOpsLaneBridge(
  __dirname,
  'moltbook_hot_collector',
  'moltbook-hot-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function invokeKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = moltbookHotBridge.run([command, `--payload-base64=${encoded}`]);
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      (out && out.stderr) || (out && out.stdout) || (out && out.payload && out.payload.error) || '',
      220
    );
    throw makeCollectorError('collector_error', detail || `moltbook_hot_collector_kernel_${command}_failed`);
  }
  const payloadOut = out && out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw makeCollectorError('collector_error', `moltbook_hot_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

function issueMoltbookApiHandle() {
  const res = issueSecretHandle({
    secret_id: 'moltbook_api_key',
    scope: 'sensory.collector.moltbook_hot',
    caller: 'adaptive/sensory/eyes/collectors/moltbook_hot',
    ttl_sec: 300,
    reason: 'collector_fetch'
  });
  return res && res.ok ? String(res.handle || '') : '';
}

function preflightMoltbookHot(eyeConfig, budgets) {
  const secret = loadSecretById('moltbook_api_key');
  return invokeKernel('preflight', {
    secret_present: secret && secret.ok === true,
    max_items: Number(budgets && budgets.max_items || 0),
    allowed_domains: Array.isArray(eyeConfig && eyeConfig.allowed_domains) ? eyeConfig.allowed_domains : [],
    host: 'www.moltbook.com',
  }, false);
}

async function collectMoltbookHot(eyeConfig, budgets) {
  const started = Date.now();
  const pf = preflightMoltbookHot(eyeConfig, budgets);
  if (!pf.ok) {
    const first = (Array.isArray(pf.failures) ? pf.failures[0] : null) || {};
    throw makeCollectorError(
      String(first.code || 'collector_preflight_failed'),
      `moltbook_hot_preflight_failed (${String(first.message || 'unknown').slice(0, 160)})`,
      { failures: Array.isArray(pf.failures) ? pf.failures.slice(0, 8) : [] }
    );
  }

  const maxItems = Math.max(1, Math.min(Number(budgets && budgets.max_items || 20), 50));
  const cacheId = eyeConfig && eyeConfig.id || 'moltbook_feed';
  const apiKeyHandle = issueMoltbookApiHandle();

  const mapped = invokeKernel('run', {
    secret_present: true,
    host: 'www.moltbook.com',
    allowed_domains: Array.isArray(eyeConfig && eyeConfig.allowed_domains) ? eyeConfig.allowed_domains : [],
    max_items: maxItems,
    api_key_handle: apiKeyHandle || null,
    timeout_ms: Math.max(2000, Math.min(Number(process.env.MOLTBOOK_HTTP_TIMEOUT_MS || 12000) || 12000, 30000)),
    topics: Array.isArray(eyeConfig && eyeConfig.topics) ? eyeConfig.topics : [],
  });
  if (mapped && mapped.success === false) {
    if (mapped.fallback_allowed === true) {
      const cached = loadCollectorCache(cacheId);
      if (cached && Array.isArray(cached.items) && cached.items.length) {
        return {
          success: true,
          items: cached.items,
          duration_ms: Date.now() - started,
          requests: 1,
          bytes: cached.items.reduce((s, it) => s + Number((it && it.bytes) || 0), 0),
          cache_hit: true
        };
      }
    }
    throw makeCollectorError(
      String(mapped.error_code || 'collector_error'),
      `moltbook_hot_fetch_failed (${String(mapped.error_code || 'collector_error')})`
    );
  }
  const items = Array.isArray(mapped && mapped.items) ? mapped.items : [];

  if (items.length > 0) {
    saveCollectorCache(cacheId, items);
  }
  return {
    success: true,
    items,
    duration_ms: Number(mapped && mapped.duration_ms || (Date.now() - started)),
    requests: Number(mapped && mapped.requests || 0),
    bytes: Number(mapped && mapped.bytes || items.reduce((s, i) => s + Number((i && i.bytes) || 0), 0))
  };
}

module.exports = { collectMoltbookHot, preflightMoltbookHot };
