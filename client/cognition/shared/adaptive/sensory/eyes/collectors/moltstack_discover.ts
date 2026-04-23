/**
 * adaptive/sensory/eyes/collectors/moltstack_discover.ts
 *
 * Thin wrapper over Rust-authoritative moltstack-discover collector kernel.
 */

const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.ts');
const {
  makeCollectorError,
} = require('./collector_errors.ts');
const { loadCollectorCache, saveCollectorCache } = require('./cache_store.ts');

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

const moltenBridge = createOpsLaneBridge(
  __dirname,
  'moltstack_discover_collector',
  'moltstack-discover-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function invokeKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = moltenBridge.run([command, `--payload-base64=${encoded}`]);
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      (out && out.stderr) || (out && out.stdout) || (out && out.payload && out.payload.error) || '',
      220
    );
    throw makeCollectorError('collector_error', detail || `moltstack_discover_collector_kernel_${command}_failed`);
  }
  const payloadOut = out && out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw makeCollectorError('collector_error', `moltstack_discover_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

function preflightMoltstackDiscover(eyeConfig, budgets) {
  const parserOptions = eyeConfig && eyeConfig.parser_options && typeof eyeConfig.parser_options === 'object'
    ? eyeConfig.parser_options
    : {};
  const url = cleanText(parserOptions.api_url, 600) || 'https://moltstack.net/api/posts';
  return invokeKernel('preflight', {
    api_url: url,
    allowed_domains: Array.isArray(eyeConfig && eyeConfig.allowed_domains)
      ? eyeConfig.allowed_domains
      : ['moltstack.net'],
    max_items: Number(budgets && budgets.max_items) || 0,
    max_seconds: Number(budgets && budgets.max_seconds) || 0,
  }, false);
}

async function collectMoltstackDiscover(eyeConfig, budgets) {
  const started = Date.now();
  const mapped = invokeKernel('run', {
    api_url: cleanText(eyeConfig && eyeConfig.parser_options && eyeConfig.parser_options.api_url, 600),
    allowed_domains: Array.isArray(eyeConfig && eyeConfig.allowed_domains)
      ? eyeConfig.allowed_domains
      : ['moltstack.net'],
    max_seconds: Number((budgets && budgets.max_seconds) || 10),
    topics: Array.isArray(eyeConfig && eyeConfig.topics) ? eyeConfig.topics : [],
    max_items: Math.max(1, Math.min(Number((budgets && budgets.max_items) || 20), 50)),
    timeout_ms: Math.max(1000, Math.min(Number((budgets && budgets.max_seconds) || 10) * 1000, 30000)),
  });

  if (mapped && mapped.success === false) {
    if (mapped.fallback_allowed === true) {
      const cached = loadCollectorCache(eyeConfig && eyeConfig.id || 'moltstack_discover');
      if (cached && Array.isArray(cached.items) && cached.items.length) {
        return {
          success: true,
          items: cached.items,
          duration_ms: Date.now() - started,
          requests: 1,
          bytes: cached.items.reduce((s, it) => s + Number((it && it.bytes) || 0), 0),
          cache_hit: true,
        };
      }
    }
    throw makeCollectorError(
      String(mapped.error_code || 'collector_error'),
      `moltstack_discover_fetch_failed (${String(mapped.error_code || 'collector_error')})`
    );
  }
  const items = Array.isArray(mapped && mapped.items) ? mapped.items : [];

  const durationMs = Date.now() - started;
  if (items.length > 0) {
    saveCollectorCache(eyeConfig && eyeConfig.id || 'moltstack_discover', items);
  }

  return {
    success: true,
    items,
    duration_ms: durationMs,
    requests: Number(mapped && mapped.requests || 0),
    bytes: Number(mapped && mapped.bytes || 0),
  };
}

module.exports = {
  collectMoltstackDiscover,
  preflightMoltstackDiscover,
};
