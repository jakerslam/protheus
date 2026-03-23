/**
 * adaptive/sensory/eyes/collectors/github_repo.ts
 *
 * GitHub repo eye and PR review runtime.
 * - Repo activity mode: release + commits + open PR headlines.
 * - PR review mode: deterministic file/diff summary for one PR.
 * - Auth modes: unauthenticated, PAT, GitHub App installation token.
 */

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
const ts = require('typescript');
const { classifyCollectorError, httpStatusToCode, makeCollectorError } = require('./collector_errors.ts');
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
const GITHUB_CACHE_DIR = path.join(EYES_STATE_DIR, 'github_repo_cache');

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

process.env.EGRESS_GATEWAY_POLICY_PATH = path.join(
  ROOT,
  'client',
  'runtime',
  'config',
  'egress_gateway_policy.json'
);

const { egressFetch, EgressGatewayError } = require('../../../../../../lib/egress_gateway.ts');

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

function cachePath(cacheKey) {
  return path.join(GITHUB_CACHE_DIR, `${cacheKey}.json`);
}

function loadRepoCache(cacheKey) {
  const base = { last_run: null, seen_ids: [] };
  const raw = readJson(cachePath(cacheKey), base);
  if (!raw || typeof raw !== 'object') return base;
  return {
    last_run: typeof raw.last_run === 'string' ? raw.last_run : null,
    seen_ids: Array.isArray(raw.seen_ids) ? raw.seen_ids.slice(-1000) : []
  };
}

function saveRepoCache(cacheKey, cache) {
  writeJson(cachePath(cacheKey), cache);
}

function sha16(s) {
  return crypto.createHash('sha256').update(String(s)).digest('hex').slice(0, 16);
}

function nowIso() {
  return new Date().toISOString();
}

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

function resolveAuth(options = {}) {
  const env = process.env;
  const appInstallationToken = cleanText(
    options.githubAppInstallationToken || options.app_installation_token || env.GITHUB_APP_INSTALLATION_TOKEN,
    400
  );
  if (appInstallationToken) {
    return {
      mode: 'github_app_installation_token',
      headers: { Authorization: `Bearer ${appInstallationToken}` }
    };
  }

  const pat = cleanText(options.githubToken || options.token || env.GITHUB_TOKEN, 400);
  if (pat) {
    return {
      mode: 'pat',
      headers: { Authorization: `Bearer ${pat}` }
    };
  }

  return { mode: 'unauthenticated', headers: {} };
}

async function fetchJson(url, { timeoutMs = 15000, auth = null, caller = 'adaptive/sensory/eyes/collectors/github_repo' } = {}) {
  try {
    const host = new URL(url).hostname;
    const authHeaders = auth && auth.headers && typeof auth.headers === 'object' ? auth.headers : {};
    const res = await egressFetch(
      url,
      {
        method: 'GET',
        headers: {
          'User-Agent': 'Infring-Eyes/1.0',
          'Accept': 'application/vnd.github+json',
          ...authHeaders
        }
      },
      {
        scope: 'sensory.collector.github_repo',
        caller,
        runtime_allowlist: [host],
        timeout_ms: timeoutMs,
        meta: { collector: 'github_repo' }
      }
    );

    const status = Number(res.status || 0);
    const text = await res.text();
    if (status >= 400) {
      throw makeCollectorError(httpStatusToCode(status), `HTTP ${status} for ${url}`, {
        http_status: status,
        url
      });
    }

    try {
      return {
        json: JSON.parse(String(text || '{}')),
        bytes: Buffer.byteLength(String(text || ''), 'utf8'),
        status
      };
    } catch {
      return {
        json: null,
        text: String(text || ''),
        bytes: Buffer.byteLength(String(text || ''), 'utf8'),
        status
      };
    }
  } catch (err) {
    if (err instanceof EgressGatewayError) {
      throw makeCollectorError(
        'env_blocked',
        `egress_denied:${String(err.decision && err.decision.reason || 'policy')} for ${url}`.slice(0, 220),
        { url }
      );
    }
    const c = classifyCollectorError(err);
    throw makeCollectorError(c.code, c.message, { url, http_status: c.http_status });
  }
}

function fileRiskFlags(files) {
  const rows = Array.isArray(files) ? files : [];
  const flags = [];
  const totalDelta = rows.reduce((sum, row) => sum + Number((row && row.changes) || 0), 0);
  if (rows.length >= 40 || totalDelta >= 2000) flags.push('large_diff');
  if (rows.some((row) => /security|auth|token|secret|vault|policy/i.test(String(row && row.filename || '')))) {
    flags.push('security_sensitive_paths');
  }
  if (rows.some((row) => /migrations?|schema|sql/i.test(String(row && row.filename || '')))) {
    flags.push('schema_or_data_migration');
  }
  return flags;
}

async function runPrReview({ owner, repo, pr, auth = null, timeoutMs = 15000 } = {}) {
  const prNumber = Number(pr);
  if (!owner || !repo || !(prNumber > 0)) {
    return { ok: false, error: 'missing_owner_repo_or_pr' };
  }

  const baseUrl = `https://api.github.com/repos/${owner}/${repo}`;
  const [prRes, filesRes] = await Promise.all([
    fetchJson(`${baseUrl}/pulls/${prNumber}`, { timeoutMs, auth, caller: 'adaptive/sensory/eyes/collectors/github_repo:pr' }),
    fetchJson(`${baseUrl}/pulls/${prNumber}/files?per_page=100`, { timeoutMs, auth, caller: 'adaptive/sensory/eyes/collectors/github_repo:pr' })
  ]);

  const prRow = prRes && prRes.json && typeof prRes.json === 'object' ? prRes.json : {};
  const files = Array.isArray(filesRes && filesRes.json) ? filesRes.json : [];
  const riskFlags = fileRiskFlags(files);

  const additions = Number(prRow.additions || 0);
  const deletions = Number(prRow.deletions || 0);
  const changedFiles = Number(prRow.changed_files || files.length || 0);
  const reviewId = sha16(`pr_review:${owner}/${repo}#${prNumber}:${prRow.head && prRow.head.sha || 'unknown'}`);

  return {
    ok: true,
    success: true,
    eye: 'github_repo',
    mode: 'pr_review',
    auth_mode: auth && auth.mode || 'unauthenticated',
    owner,
    repo,
    pr: prNumber,
    review: {
      id: reviewId,
      title: cleanText(prRow.title, 220),
      url: cleanText(prRow.html_url, 500),
      state: cleanText(prRow.state, 40),
      draft: prRow.draft === true,
      author: cleanText(prRow.user && prRow.user.login, 120),
      files_changed: changedFiles,
      additions,
      deletions,
      risk_flags: riskFlags,
      file_sample: files.slice(0, 8).map((row) => ({
        filename: cleanText(row && row.filename, 200),
        status: cleanText(row && row.status, 40),
        additions: Number((row && row.additions) || 0),
        deletions: Number((row && row.deletions) || 0),
        changes: Number((row && row.changes) || 0)
      }))
    },
    bytes: Number(prRes.bytes || 0) + Number(filesRes.bytes || 0),
    requests: 2,
    duration_ms: 0
  };
}

async function runRepoActivity({ owner, repo, maxItems = 10, minHours = 4, force = false, auth = null, timeoutMs = 15000 } = {}) {
  const cacheKey = `github_repo_${owner}_${repo}`;
  const cache = loadRepoCache(cacheKey);
  const lastRun = cache.last_run ? new Date(cache.last_run) : null;
  const hoursSince = lastRun ? (Date.now() - lastRun.getTime()) / (1000 * 60 * 60) : Infinity;

  if (!force && hoursSince < minHours) {
    return {
      ok: true,
      skipped: true,
      reason: 'cadence',
      hours_since_last: Number(hoursSince.toFixed(2)),
      min_hours: minHours
    };
  }

  const items = [];
  let bytes = 0;
  const baseUrl = `https://api.github.com/repos/${owner}/${repo}`;

  // Latest release
  try {
    const { json: release, bytes: releaseBytes } = await fetchJson(`${baseUrl}/releases/latest`, {
      timeoutMs,
      auth,
      caller: 'adaptive/sensory/eyes/collectors/github_repo'
    });
    bytes += releaseBytes;
    if (release && release.tag_name) {
      const id = sha16(`release-${owner}-${repo}-${release.tag_name}`);
      if (!cache.seen_ids?.includes(id)) {
        items.push({
          id,
          collected_at: nowIso(),
          url: cleanText(release.html_url, 500),
          title: `${owner}/${repo}: ${cleanText(release.tag_name, 80)}`,
          description: cleanText(`Release: ${release.name || release.tag_name}. ${release.body || ''}`, 280),
          type: 'release',
          tag_name: cleanText(release.tag_name, 80),
          published_at: cleanText(release.published_at, 120) || null,
          author: cleanText(release.author && release.author.login, 120),
          signal_type: 'repo_release',
          signal: true,
          source: 'github_repo',
          repo: `${owner}/${repo}`,
          tags: ['github', 'release', 'software'],
          topics: ['repo_activity', 'releases'],
          bytes: releaseBytes
        });
      }
    }
  } catch {
    // Releases are optional for many repositories.
  }

  // Recent commits
  try {
    const { json: commits, bytes: commitBytes } = await fetchJson(`${baseUrl}/commits?per_page=5`, {
      timeoutMs,
      auth,
      caller: 'adaptive/sensory/eyes/collectors/github_repo'
    });
    bytes += commitBytes;
    if (Array.isArray(commits)) {
      for (const commit of commits.slice(0, 3)) {
        const id = sha16(`commit-${owner}-${repo}-${commit && commit.sha}`);
        if (cache.seen_ids?.includes(id)) continue;
        items.push({
          id,
          collected_at: nowIso(),
          url: cleanText(commit && commit.html_url, 500),
          title: `${owner}/${repo}: ${cleanText(commit && commit.commit && commit.commit.message && commit.commit.message.split('\n')[0], 90)}`,
          description: cleanText(`Commit by ${commit && commit.commit && commit.commit.author && commit.commit.author.name || 'unknown'}`, 220),
          type: 'commit',
          sha: cleanText(commit && commit.sha, 16),
          author: cleanText(commit && commit.commit && commit.commit.author && commit.commit.author.name, 120),
          date: cleanText(commit && commit.commit && commit.commit.author && commit.commit.author.date, 120),
          signal_type: 'repo_commit',
          signal: false,
          source: 'github_repo',
          repo: `${owner}/${repo}`,
          tags: ['github', 'commit'],
          topics: ['repo_activity', 'development'],
          bytes: 0
        });
      }
    }
  } catch {
    // Continue without commits.
  }

  // Open PR headlines (code-review lane ingress)
  try {
    const { json: pulls, bytes: pullBytes } = await fetchJson(`${baseUrl}/pulls?state=open&per_page=5`, {
      timeoutMs,
      auth,
      caller: 'adaptive/sensory/eyes/collectors/github_repo:pr_index'
    });
    bytes += pullBytes;
    if (Array.isArray(pulls)) {
      for (const pr of pulls.slice(0, 3)) {
        const number = Number(pr && pr.number || 0);
        if (!(number > 0)) continue;
        const id = sha16(`pr-${owner}-${repo}-${number}-${pr && pr.updated_at || ''}`);
        if (cache.seen_ids?.includes(id)) continue;
        items.push({
          id,
          collected_at: nowIso(),
          url: cleanText(pr && pr.html_url, 500),
          title: `${owner}/${repo} PR #${number}: ${cleanText(pr && pr.title, 120)}`,
          description: cleanText(`Open PR by ${pr && pr.user && pr.user.login || 'unknown'}; draft=${pr && pr.draft === true}`, 220),
          type: 'pull_request',
          pr: number,
          author: cleanText(pr && pr.user && pr.user.login, 120),
          date: cleanText(pr && pr.updated_at, 120),
          signal_type: 'repo_pr_open',
          signal: true,
          source: 'github_repo',
          repo: `${owner}/${repo}`,
          tags: ['github', 'pull_request'],
          topics: ['code_review', 'repo_activity'],
          bytes: 0
        });
      }
    }
  } catch {
    // Continue without PR index.
  }

  cache.last_run = nowIso();
  cache.seen_ids = [...(cache.seen_ids || []).slice(-500), ...items.map((i) => i.id)];
  saveRepoCache(cacheKey, cache);

  return {
    ok: true,
    success: true,
    eye: 'github_repo',
    mode: 'repo_activity',
    auth_mode: auth && auth.mode || 'unauthenticated',
    owner,
    repo,
    items: items.slice(0, Number(maxItems) || 10),
    bytes,
    duration_ms: 0,
    requests: 3,
    cadence_hours: minHours,
    sample: items[0] && items[0].type ? items[0].type : null
  };
}

async function run(options = {}) {
  const owner = cleanText(options.owner, 160);
  const repo = cleanText(options.repo, 160);
  const pr = options.pr == null ? null : Number(options.pr);
  const maxItems = Number(options.maxItems || options.max_items || 10);
  const minHours = Number(options.minHours || options.min_hours || 4);
  const force = boolFlag(options.force, false);
  const timeoutMs = Number(options.timeoutMs || options.timeout_ms || 15000);

  if (!owner || !repo) {
    return { ok: false, success: false, eye: 'github_repo', error: 'missing_owner_or_repo' };
  }

  try {
    const auth = resolveAuth(options);
    if (pr && pr > 0) {
      return runPrReview({ owner, repo, pr, auth, timeoutMs });
    }
    return runRepoActivity({ owner, repo, maxItems, minHours, force, auth, timeoutMs });
  } catch (err) {
    return {
      ok: false,
      success: false,
      eye: 'github_repo',
      owner,
      repo,
      mode: pr && pr > 0 ? 'pr_review' : 'repo_activity',
      items: [],
      bytes: 0,
      duration_ms: 0,
      requests: 1,
      cadence_hours: minHours,
      error: err && (err.code || err.message) || 'collector_error'
    };
  }
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
      process.exit(r.ok ? 0 : 1);
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
