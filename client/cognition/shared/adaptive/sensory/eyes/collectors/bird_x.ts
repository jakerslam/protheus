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

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

const birdBridge = createOpsLaneBridge(
  __dirname,
  'bird_x_collector',
  'bird-x-collector-kernel',
  { preferLocalCore: true }
);

function parseBoolToken(value) {
  const text = String(value == null ? '' : value).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(text)) return true;
  if (['0', 'false', 'no', 'off'].includes(text)) return false;
  return null;
}

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function parseArgs(argv = []) {
  const out = { command: 'collect', force: false, queries: [] };
  let consumedCommand = false;
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (!s) continue;
    if (!consumedCommand && !s.startsWith('--')) {
      if (s === 'preflight' || s === 'collect') {
        out.command = s;
        consumedCommand = true;
        continue;
      }
      consumedCommand = true;
      out.queries.push(cleanText(s, 200));
      continue;
    }
    if (s === '--force') out.force = true;
    else if (s.startsWith('--max=')) out.maxItems = Number(s.slice('--max='.length));
    else if (s.startsWith('--min-hours=')) out.minHours = Number(s.slice('--min-hours='.length));
    else if (s.startsWith('--max-per-query=')) {
      out.maxItemsPerQuery = Number(s.slice('--max-per-query='.length));
    } else if (s.startsWith('--timeout-ms=')) {
      out.timeoutMs = Number(s.slice('--timeout-ms='.length));
    } else if (s.startsWith('--retry-attempts=')) {
      out.retryAttempts = Number(s.slice('--retry-attempts='.length));
    } else if (s.startsWith('--bird-cli-present=')) {
      out.birdCliPresent = parseBoolToken(s.slice('--bird-cli-present='.length));
    } else if (s.startsWith('--query=')) {
      const query = cleanText(s.slice('--query='.length), 200);
      if (query) out.queries.push(query);
    }
  }
  out.queries = out.queries.filter(Boolean);
  return out;
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

async function preflightBirdX(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  const payload = {};
  if (typeof opts.birdCliPresent === 'boolean') {
    payload.bird_cli_present = opts.birdCliPresent;
  }
  return invokeBirdKernel('preflight', payload, false);
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
    bird_cli_present: typeof options.birdCliPresent === 'boolean' ? options.birdCliPresent : undefined,
  });
}

async function run(input = {}) {
  const opts = Array.isArray(input)
    ? parseArgs(input)
    : (input && typeof input === 'object' ? input : {});
  if (opts.command === 'preflight') {
    return preflightBirdX(opts);
  }
  return collectBirdX(opts);
}

module.exports = {
  parseArgs,
  run,
  collectBirdX,
  preflightBirdX,
};
