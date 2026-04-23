/**
 * adaptive/sensory/eyes/collectors/stock_market.ts
 *
 * Thin wrapper over Rust-authoritative stock-market collector kernel.
 * All fetch, cadence, fallback, and mapping authority resides in Rust.
 */

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.ts');
const { makeCollectorError } = require('./collector_errors.ts');

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
const EYES_STATE_DIR = process.env.EYES_STATE_DIR
  ? path.resolve(process.env.EYES_STATE_DIR)
  : path.join(WORKSPACE_ROOT, 'local', 'state', 'sensory', 'eyes');

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

const stockMarketBridge = createOpsLaneBridge(
  __dirname,
  'stock_market_collector',
  'stock-market-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function clampInt(value, min, max, fallback) {
  const n = Number(value);
  if (!Number.isFinite(n)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(n)));
}

function nowIso() {
  return new Date().toISOString();
}

function invokeStockKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = stockMarketBridge.run([command, `--payload-base64=${encoded}`]);
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      (out && out.stderr) || (out && out.stdout) || (out && out.payload && out.payload.error) || '',
      220
    );
    throw makeCollectorError('collector_error', detail || `stock_market_collector_kernel_${command}_failed`);
  }
  const payloadOut = out && out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw makeCollectorError('collector_error', `stock_market_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

async function run({ maxItems = 20, minHours = 1, force = false, timeoutMs = 15000 } = {}) {
  return invokeStockKernel('run', {
    eyes_state_dir: EYES_STATE_DIR,
    force: force === true,
    min_hours: Number.isFinite(Number(minHours)) ? Number(minHours) : 1,
    max_items: clampInt(maxItems, 1, 200, 20),
    timeout_ms: clampInt(timeoutMs, 1000, 120000, 15000),
  });
}

function extractQuotesFromHtml(html) {
  const out = invokeStockKernel('extract-quotes', { html: String(html || '') });
  return Array.isArray(out && out.quotes) ? out.quotes : [];
}

function buildFallbackIndices(options = {}) {
  const out = invokeStockKernel('fallback-indices', {
    max_items: clampInt(options.maxItems, 1, 200, 20),
    seen_ids: Array.isArray(options.seenIds) ? options.seenIds : [],
    date: cleanText(options.date, 32) || nowIso().slice(0, 10),
  });
  return Array.isArray(out && out.items) ? out.items : [];
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const maxItems = Number(args.find((a) => a.startsWith('--max='))?.split('=')[1] || 20);
  const minHours = Number(args.find((a) => a.startsWith('--min-hours='))?.split('=')[1] || 1);
  const timeoutMs = Number(args.find((a) => a.startsWith('--timeout-ms='))?.split('=')[1] || 15000);
  const force = args.includes('--force');

  run({ maxItems, minHours, force, timeoutMs })
    .then((r) => {
      console.log(JSON.stringify(r));
      process.exit(r && r.ok ? 0 : 1);
    })
    .catch((e) => {
      console.error(JSON.stringify({ ok: false, error: e && e.message ? e.message : 'collector_error' }));
      process.exit(1);
    });
}

module.exports = { run, extractQuotesFromHtml, buildFallbackIndices };
