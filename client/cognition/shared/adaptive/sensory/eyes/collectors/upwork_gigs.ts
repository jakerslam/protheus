/**
 * adaptive/sensory/eyes/collectors/upwork_gigs.ts
 *
 * Thin wrapper over Rust-authoritative upwork-gigs collector kernel.
 */

const fs = require('fs');
const path = require('path');
const { createOpsLaneBridge } = require('../../../../../../runtime/lib/rust_lane_bridge.js');

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

const upworkBridge = createOpsLaneBridge(
  __dirname,
  'upwork_gigs_collector',
  'upwork-gigs-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .trim()
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {}
  }
  return null;
}

function invokeUpworkKernel(command, payload = {}, requireOk = true) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = upworkBridge.run([command, `--payload-base64=${encoded}`]);
  const parsed =
    out && out.payload && typeof out.payload === 'object'
      ? out.payload
      : parseLastJson(String((out && out.stdout) || ''));
  const status = Number.isFinite(Number(out && out.status)) ? Number(out.status) : 1;
  if (!parsed || status !== 0) {
    throw new Error(`upwork_gigs_collector_kernel_${command}_failed:${status}`);
  }
  const payloadOut = parsed && parsed.payload && typeof parsed.payload === 'object'
    ? parsed.payload
    : parsed;
  if (!payloadOut || (requireOk && payloadOut.ok !== true)) {
    throw new Error(`upwork_gigs_collector_kernel_${command}_invalid_payload`);
  }
  return payloadOut;
}

async function run({ maxItems = 10, minHours = 4, force = false, timeoutMs = 15000, searchQuery = '' } = {}) {
  const maxItemsNorm = Number.isFinite(Number(maxItems)) ? Number(maxItems) : 10;
  const minHoursNorm = Number.isFinite(Number(minHours)) ? Number(minHours) : 4;
  const timeoutMsNorm = Number.isFinite(Number(timeoutMs)) ? Number(timeoutMs) : 15000;
  return invokeUpworkKernel('run', {
    eyes_state_dir: EYES_STATE_DIR,
    force: force === true,
    min_hours: minHoursNorm,
    max_items: maxItemsNorm,
    timeout_ms: timeoutMsNorm,
    search_query: cleanText(searchQuery, 240)
  });
}

if (require.main === module) {
  const args = process.argv.slice(2);
  const maxArg = args.find((a) => a.startsWith('--max='));
  const minArg = args.find((a) => a.startsWith('--min-hours='));
  const timeoutArg = args.find((a) => a.startsWith('--timeout-ms='));
  const maxItems = Number(maxArg ? maxArg.split('=')[1] : 10);
  const minHours = Number(minArg ? minArg.split('=')[1] : 4);
  const timeoutMs = Number(timeoutArg ? timeoutArg.split('=')[1] : 15000);
  const searchQuery = String(args.find((a) => a.startsWith('--q='))?.split('=')[1] || '');
  const force = args.includes('--force');

  run({ maxItems, minHours, timeoutMs, force, searchQuery })
    .then((r) => {
      console.log(JSON.stringify(r));
      process.exit(r && r.ok ? 0 : 1);
    })
    .catch((e) => {
      console.error(JSON.stringify({ ok: false, error: e && e.message ? e.message : 'collector_error' }));
      process.exit(1);
    });
}

module.exports = { run };
