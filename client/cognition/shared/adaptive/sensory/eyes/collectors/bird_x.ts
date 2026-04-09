/**
 * adaptive/sensory/eyes/collectors/bird_x.ts
 *
 * Thin wrapper over Rust-authoritative bird-x collector kernel.
 * Client side keeps only bridge transport.
 */

const fs = require('fs');
const path = require('path');
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
const EYES_STATE_DIR = process.env.EYES_STATE_DIR
  ? path.resolve(process.env.EYES_STATE_DIR)
  : path.join(WORKSPACE_ROOT, 'local', 'state', 'sensory', 'eyes');

process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';

const birdBridge = createOpsLaneBridge(
  __dirname,
  'bird_x_collector',
  'bird-x-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function invokeBirdKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = birdBridge.run([command, `--payload-base64=${encoded}`]);
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (status !== 0) {
    const detail = cleanText(
      (out && out.stderr)
      || (out && out.stdout)
      || (out && out.payload && out.payload.error)
      || '',
      220
    );
    throw new Error(detail || `bird_x_collector_kernel_${command}_failed`);
  }
  const payloadOut = out && out.payload && out.payload.payload && typeof out.payload.payload === 'object'
    ? out.payload.payload
    : null;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw new Error(`bird_x_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

async function preflightBirdX() {
  return invokeBirdKernel('preflight', {}, false);
}

async function collectBirdX(options = {}) {
  const retryAttempts = Number(
    options.retryAttempts == null
      ? process.env.EYES_COLLECTOR_FETCH_RETRY_ATTEMPTS
      : options.retryAttempts
  );
  return invokeBirdKernel('collect', {
    eyes_state_dir: EYES_STATE_DIR,
    force: options.force === true,
    min_hours: Number.isFinite(Number(options.minHours)) ? Number(options.minHours) : 0,
    max_items: Number.isFinite(Number(options.maxItems)) ? Number(options.maxItems) : 15,
    max_items_per_query: Number.isFinite(Number(options.maxItemsPerQuery))
      ? Number(options.maxItemsPerQuery)
      : 10,
    timeout_ms: Number.isFinite(Number(options.timeoutMs)) ? Number(options.timeoutMs) : 15000,
    retry_attempts: Number.isFinite(retryAttempts) ? retryAttempts : 2,
    queries: Array.isArray(options.queries)
      ? options.queries.map((q) => cleanText(q, 200)).filter(Boolean)
      : [],
  });
}

module.exports = {
  collectBirdX,
  preflightBirdX,
};
