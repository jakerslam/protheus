/**
 * adaptive/sensory/eyes/collectors/github_repo.ts
 *
 * Thin transport wrapper over Rust-authoritative github-repo collector kernel.
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

const ROOT = resolveWorkspaceRoot();
const EYES_STATE_DIR = process.env.EYES_STATE_DIR
  ? path.resolve(process.env.EYES_STATE_DIR)
  : path.join(ROOT, 'local', 'state', 'sensory', 'eyes');

process.env.INFRING_OPS_USE_PREBUILT = '0';
process.env.INFRING_OPS_LOCAL_TIMEOUT_MS = process.env.INFRING_OPS_LOCAL_TIMEOUT_MS || '120000';

const githubRepoKernelBridge = createOpsLaneBridge(
  __dirname,
  'github_repo',
  'github-repo-collector-kernel',
  { preferLocalCore: true }
);

function cleanText(v, max = 240) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, max);
}

function boolFlag(v, fallback = false) {
  if (v == null) return fallback;
  const s = String(v).trim().toLowerCase();
  if (!s) return fallback;
  return s === '1' || s === 'true' || s === 'yes' || s === 'on';
}

function parseArgs(argv = []) {
  const out = { force: false };
  for (const token of Array.isArray(argv) ? argv : []) {
    const s = String(token || '');
    if (!s.startsWith('--')) continue;
    if (s === '--force') {
      out.force = true;
      continue;
    }
    const idx = s.indexOf('=');
    if (idx > 2) {
      out[s.slice(2, idx)] = s.slice(idx + 1);
    } else {
      out[s.slice(2)] = true;
    }
  }
  return out;
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

function runKernel(command, payload = {}) {
  const encoded = Buffer.from(JSON.stringify(payload), 'utf8').toString('base64');
  const out = githubRepoKernelBridge.run([command, `--payload-base64=${encoded}`]);
  const parsed =
    out && out.payload && typeof out.payload === 'object'
      ? out.payload
      : parseLastJson(String((out && out.stdout) || ''));
  const status = out && Number.isFinite(Number(out.status)) ? Number(out.status) : 1;
  if (!parsed || status !== 0) {
    throw new Error(`github_repo_kernel_failed:${status}`);
  }
  const payloadOut = (parsed.payload && typeof parsed.payload === 'object')
    ? parsed.payload
    : parsed;
  return payloadOut;
}

function resolveAuth(options = {}) {
  return runKernel('resolve-auth', {
    github_app_installation_token: options.githubAppInstallationToken || options.app_installation_token,
    app_installation_token: options.app_installation_token,
    github_token: options.githubToken || options.token,
    token: options.token
  });
}

function fileRiskFlags(files) {
  const out = runKernel('file-risk-flags', {
    files: Array.isArray(files) ? files : []
  });
  return Array.isArray(out && out.risk_flags) ? out.risk_flags : [];
}

async function run(options = {}) {
  const opts = options && typeof options === 'object' ? options : {};
  try {
    return runKernel('run', {
      owner: opts.owner,
      repo: opts.repo,
      pr: opts.pr,
      max_items: opts.maxItems || opts.max_items,
      min_hours: opts.minHours || opts.min_hours,
      force: boolFlag(opts.force, false),
      timeout_ms: opts.timeoutMs || opts.timeout_ms,
      state_dir: EYES_STATE_DIR,
      github_app_installation_token: opts.githubAppInstallationToken || opts.app_installation_token,
      app_installation_token: opts.app_installation_token,
      github_token: opts.githubToken || opts.token,
      token: opts.token
    });
  } catch (err) {
    return {
      ok: false,
      success: false,
      eye: 'github_repo',
      mode: Number(opts.pr) > 0 ? 'pr_review' : 'repo_activity',
      items: [],
      bytes: 0,
      duration_ms: 0,
      requests: 1,
      cadence_hours: Number.isFinite(Number(opts.minHours ?? opts.min_hours))
        ? Number(opts.minHours ?? opts.min_hours)
        : 4,
      error: cleanText(err && (err.code || err.message) || 'collector_error', 160)
    };
  }
}

async function runPrReview({ owner, repo, pr, auth = null, timeoutMs = 15000 } = {}) {
  return run({
    owner,
    repo,
    pr,
    timeoutMs,
    githubAppInstallationToken: auth && auth.headers && auth.headers.Authorization
      ? String(auth.headers.Authorization).replace(/^Bearer\s+/i, '')
      : undefined
  });
}

async function runRepoActivity({ owner, repo, maxItems = 10, minHours = 4, force = false, auth = null, timeoutMs = 15000 } = {}) {
  return run({
    owner,
    repo,
    maxItems,
    minHours,
    force,
    timeoutMs,
    githubAppInstallationToken: auth && auth.headers && auth.headers.Authorization
      ? String(auth.headers.Authorization).replace(/^Bearer\s+/i, '')
      : undefined
  });
}

if (require.main === module) {
  const args = parseArgs(process.argv.slice(2));
  const owner = cleanText(args.owner, 160);
  const repo = cleanText(args.repo, 160);
  if (!owner || !repo) {
    console.error(JSON.stringify({ ok: false, error: 'Missing --owner or --repo' }));
    process.exit(1);
  }
  run({
    owner,
    repo,
    pr: args.pr,
    maxItems: args.max,
    minHours: args['min-hours'],
    force: args.force,
    githubToken: args.token,
    githubAppInstallationToken: args['app-installation-token']
  })
    .then((r) => {
      console.log(JSON.stringify(r));
      process.exit(r && r.ok ? 0 : 1);
    })
    .catch((e) => {
      console.error(JSON.stringify({ ok: false, error: e && e.message || 'collector_error' }));
      process.exit(1);
    });
}

module.exports = {
  run,
  runPrReview,
  runRepoActivity,
  resolveAuth,
  parseArgs,
  fileRiskFlags
};
