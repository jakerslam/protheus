/**
 * adaptive/sensory/eyes/collectors/local_state_digest.ts
 *
 * Thin wrapper over Rust-authoritative local-state-digest kernel.
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

const WORKSPACE_DIR = resolveWorkspaceRoot();
const STATE_DIR = path.join(WORKSPACE_DIR, 'local', 'state');

process.env.PROTHEUS_OPS_USE_PREBUILT = '0';
process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS = process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000';

const localStateDigestBridge = createOpsLaneBridge(
  __dirname,
  'local_state_digest',
  'local-state-digest-kernel',
  { preferLocalCore: true }
);

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

function runKernel(command, payload = {}) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = localStateDigestBridge.run([command, `--payload-base64=${encoded}`]);
  const parsed =
    out && out.payload && typeof out.payload === 'object'
      ? out.payload
      : parseLastJson(String((out && out.stdout) || ''));
  const status = out && Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (!parsed || status !== 0) {
    throw new Error(`local_state_digest_kernel_failed:${status}`);
  }
  const payloadOut = (parsed.payload && typeof parsed.payload === 'object')
    ? parsed.payload
    : parsed;
  return payloadOut;
}

function preflightLocalStateDigest(eyeConfig, budgets) {
  return runKernel('preflight', {
    eye_config: eyeConfig && typeof eyeConfig === 'object' ? eyeConfig : {},
    budgets: budgets && typeof budgets === 'object' ? budgets : {},
    state_dir: STATE_DIR,
  });
}

async function collectLocalStateDigest(eyeConfig, budgets) {
  const pf = preflightLocalStateDigest(eyeConfig, budgets);
  if (!pf || pf.ok !== true) {
    const first = Array.isArray(pf && pf.failures) ? pf.failures[0] : null;
    const code = String((first && first.code) || 'local_state_preflight_failed');
    const message = String((first && first.message) || 'unknown').slice(0, 160);
    const err = new Error(`local_state_preflight_failed (${message})`);
    err.code = code;
    throw err;
  }

  const out = runKernel('collect', {
    eye_config: eyeConfig && typeof eyeConfig === 'object' ? eyeConfig : {},
    budgets: budgets && typeof budgets === 'object' ? budgets : {},
    state_dir: STATE_DIR,
  });

  if (out && out.ok === false && out.error) {
    const code = String(out.error.code || 'collector_error');
    const message = String(out.error.message || code).slice(0, 220);
    const err = new Error(message);
    err.code = code;
    throw err;
  }

  return out;
}

module.exports = {
  collectLocalStateDigest,
  preflightLocalStateDigest,
};
