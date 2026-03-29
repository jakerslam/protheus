#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Svelte dashboard over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { spawn, spawnSync } = require('node:child_process');
const { ROOT, resolveBinary, runProtheusOps } = require('../ops/run_protheus_ops.ts');

const DASHBOARD_DIR = __dirname;
const SVELTE_DIR = path.resolve(DASHBOARD_DIR, 'svelte_dashboard');
const SVELTE_DIST_DIR = path.resolve(SVELTE_DIR, 'dist');
const STATUS_DIR = path.resolve(ROOT, 'client/runtime/local/state/ui/infring_dashboard');
const STATUS_PATH = path.resolve(STATUS_DIR, 'server_status.json');
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 2000;
const DEFAULT_BACKEND_READY_TIMEOUT_MS = 120000;
const BACKEND_PORT_OFFSET = 1000;
const HOP_BY_HOP = new Set(['connection', 'host', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade']);
const MIME = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.md': 'text/plain; charset=utf-8',
  '.mp3': 'audio/mpeg',
  '.ogg': 'audio/ogg',
  '.pdf': 'application/pdf',
  '.png': 'image/png',
  '.svg': 'image/svg+xml; charset=utf-8',
  '.txt': 'text/plain; charset=utf-8',
  '.wav': 'audio/wav',
  '.webm': 'audio/webm',
  '.webp': 'image/webp',
  '.woff': 'font/woff',
  '.woff2': 'font/woff2',
};

function nowIso() { return new Date().toISOString(); }
function cleanText(value, maxLen = 200) { return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function isTransientSocketError(error) {
  const code = cleanText(error && error.code ? error.code : '', 40);
  return code === 'ECONNRESET' || code === 'EPIPE' || code === 'ERR_STREAM_PREMATURE_CLOSE';
}
function ignoreStreamErrors(stream) {
  if (!stream || typeof stream.on !== 'function') return;
  if (stream.__infringIgnoreErrorsInstalled) return;
  stream.__infringIgnoreErrorsInstalled = true;
  stream.on('error', () => {});
}
function parsePositiveInt(value, fallback, min = 1, max = 65535) {
  const num = Number(value);
  if (!Number.isFinite(num)) return fallback;
  return Math.max(min, Math.min(max, Math.floor(num)));
}
function normalizeArgs(argv = process.argv.slice(2)) { return Array.isArray(argv) ? argv.map((token) => String(token || '').trim()).filter(Boolean) : []; }
function defaultApiPort(port) {
  if (port + BACKEND_PORT_OFFSET <= 65535) return port + BACKEND_PORT_OFFSET;
  if (port - BACKEND_PORT_OFFSET >= 1) return port - BACKEND_PORT_OFFSET;
  return port === 65535 ? 65534 : port + 1;
}
function parseFlags(argv = []) {
  const out = { mode: 'serve', host: DEFAULT_HOST, port: DEFAULT_PORT, team: DEFAULT_TEAM, refreshMs: DEFAULT_REFRESH_MS, pretty: true, apiHost: '', apiPort: 0, apiReadyTimeoutMs: DEFAULT_BACKEND_READY_TIMEOUT_MS };
  let modeSet = false;
  for (const token of argv) {
    const value = String(token || '').trim();
    if (!value) continue;
    if (!modeSet && !value.startsWith('--')) { out.mode = value.toLowerCase(); modeSet = true; continue; }
    if (value.startsWith('--host=')) out.host = cleanText(value.slice(7), 100) || DEFAULT_HOST;
    else if (value.startsWith('--port=')) out.port = parsePositiveInt(value.slice(7), DEFAULT_PORT);
    else if (value.startsWith('--team=')) out.team = cleanText(value.slice(7), 80) || DEFAULT_TEAM;
    else if (value.startsWith('--refresh-ms=')) out.refreshMs = parsePositiveInt(value.slice(13), DEFAULT_REFRESH_MS, 800, 60000);
    else if (value.startsWith('--api-host=')) out.apiHost = cleanText(value.slice(11), 100);
    else if (value.startsWith('--backend-host=')) out.apiHost = cleanText(value.slice(15), 100);
    else if (value.startsWith('--api-port=')) out.apiPort = parsePositiveInt(value.slice(11), 0);
    else if (value.startsWith('--backend-port=')) out.apiPort = parsePositiveInt(value.slice(15), 0);
    else if (value.startsWith('--api-ready-timeout-ms=')) out.apiReadyTimeoutMs = parsePositiveInt(value.slice(23), DEFAULT_BACKEND_READY_TIMEOUT_MS, 1500, 300000);
    else if (value === '--pretty=0' || value === '--pretty=false') out.pretty = false;
  }
  out.apiHost = out.apiHost || out.host;
  out.apiPort = out.apiPort || defaultApiPort(out.port);
  if (out.apiPort === out.port) out.apiPort = defaultApiPort(out.port + 1);
  return out;
}
function ensureDir(dirPath) { fs.mkdirSync(dirPath, { recursive: true }); }
function writeJson(filePath, value) { ensureDir(path.dirname(filePath)); fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8'); }
function fileExists(filePath) { try { return fs.existsSync(filePath); } catch { return false; } }
function hasSvelteDashboardUi() {
  const indexPath = path.resolve(SVELTE_DIST_DIR, 'index.html');
  return fileExists(indexPath);
}
function ensureSvelteDashboardUiAvailable() {
  if (hasSvelteDashboardUi()) return;
  const build = spawnSync('npm', ['run', '-s', 'dashboard:svelte:build'], {
    cwd: ROOT,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
    timeout: 180000,
    env: { ...process.env, PROTHEUS_ROOT: ROOT },
  });
  if (build.stdout) process.stdout.write(build.stdout);
  if (build.stderr) process.stderr.write(build.stderr);
  if (build.status !== 0) {
    const message = cleanText(build.stderr || build.stdout || `exit_${build.status}`, 220);
    throw new Error(`svelte_dashboard_build_failed:${message}`);
  }
  if (!hasSvelteDashboardUi()) throw new Error('svelte_dashboard_ui_missing_after_build');
}
function contentTypeForFile(filePath) { return MIME[path.extname(filePath).toLowerCase()] || 'application/octet-stream'; }
function readSvelteDashboardAsset(pathname) {
  let requestPath = pathname;
  if (pathname === '/' || pathname === '/dashboard' || pathname.startsWith('/dashboard/')) requestPath = '/index.html';
  else if (pathname === '/svelte' || pathname === '/svelte/') requestPath = '/index.html';
  else if (pathname.startsWith('/svelte/')) requestPath = pathname.slice('/svelte'.length);
  else return null;
  const normalized = requestPath.startsWith('/') ? requestPath : `/${requestPath}`;
  let resolved = path.resolve(SVELTE_DIST_DIR, normalized.replace(/^\/+/, ''));
  if (!resolved.startsWith(SVELTE_DIST_DIR)) return null;
  if (!fileExists(resolved)) return null;
  try {
    if (fs.statSync(resolved).isDirectory()) {
      resolved = path.resolve(resolved, 'index.html');
      if (!resolved.startsWith(SVELTE_DIST_DIR) || !fileExists(resolved)) return null;
    }
  } catch {
    return null;
  }
  const ext = path.extname(resolved).toLowerCase();
  if (['.js', '.css', '.html', '.json', '.txt', '.map'].includes(ext)) {
    return { body: fs.readFileSync(resolved, 'utf8'), contentType: contentTypeForFile(resolved) };
  }
  return { body: fs.readFileSync(resolved), contentType: contentTypeForFile(resolved) };
}
function backendBase(flags) { return `http://${flags.apiHost}:${flags.apiPort}`; }
async function sleep(ms) { await new Promise((resolve) => setTimeout(resolve, ms)); }
async function fetchBackend(flags, pathname, init = {}, timeoutMs = 15000) {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), timeoutMs);
  try { return await fetch(`${backendBase(flags)}${pathname}`, { ...init, signal: controller.signal }); }
  finally { clearTimeout(timer); }
}
async function fetchBackendJson(flags, pathname, timeoutMs = 15000) {
  const res = await fetchBackend(flags, pathname, { cache: 'no-store' }, timeoutMs);
  if (!res.ok) throw new Error(`backend_http_${pathname}_${res.status}`);
  return res.json();
}
async function backendHealth(flags, timeoutMs = 5000) {
  try { return (await fetchBackend(flags, '/healthz', {}, timeoutMs)).ok; } catch { return false; }
}
function spawnBackend(flags) {
  const laneArgs = ['dashboard-ui', 'serve', `--host=${flags.apiHost}`, `--port=${flags.apiPort}`, `--team=${flags.team}`, `--refresh-ms=${flags.refreshMs}`];
  const env = { ...process.env, PROTHEUS_ROOT: ROOT };
  const bin = resolveBinary({ env });
  const child = bin
    ? spawn(bin, laneArgs, { cwd: ROOT, env, stdio: ['ignore', 'pipe', 'pipe'] })
    : spawn('cargo', ['run', '--quiet', '-p', 'protheus-ops-core', '--bin', 'protheus-ops', '--', ...laneArgs], { cwd: ROOT, env, stdio: ['ignore', 'pipe', 'pipe'] });
  if (child.stdout) child.stdout.on('data', (chunk) => process.stdout.write(chunk));
  if (child.stderr) child.stderr.on('data', (chunk) => process.stderr.write(chunk));
  return child;
}
async function ensureBackend(flags) {
  if (await backendHealth(flags, 1500)) return { child: null, reused: true };
  const child = spawnBackend(flags);
  const deadline = Date.now() + flags.apiReadyTimeoutMs;
  while (Date.now() < deadline) {
    if (await backendHealth(flags, 1500)) return { child, reused: false };
    if (child.exitCode != null) throw new Error(`dashboard_backend_exit:${child.exitCode}`);
    await sleep(250);
  }
  try { child.kill('SIGTERM'); } catch {}
  throw new Error('dashboard_backend_timeout');
}
function sendJson(res, statusCode, value) {
  res.writeHead(statusCode, { 'content-type': 'application/json; charset=utf-8', 'cache-control': 'no-store' });
  res.end(`${JSON.stringify(value, null, 2)}\n`);
}
function filteredHeaders(headers, host) {
  const out = {};
  for (const [key, value] of Object.entries(headers || {})) {
    if (!value || HOP_BY_HOP.has(String(key).toLowerCase())) continue;
    out[key] = value;
  }
  out.host = host;
  return out;
}
function proxyToBackend(req, res, flags) {
  return new Promise((resolve, reject) => {
    ignoreStreamErrors(req);
    ignoreStreamErrors(res);
    ignoreStreamErrors(req.socket);
    ignoreStreamErrors(res.socket);
    const upstream = http.request({ host: flags.apiHost, port: flags.apiPort, method: req.method || 'GET', path: req.url || '/', headers: filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`) }, (upstreamRes) => {
      ignoreStreamErrors(upstreamRes);
      ignoreStreamErrors(upstreamRes.socket);
      res.writeHead(upstreamRes.statusCode || 502, upstreamRes.headers);
      upstreamRes.pipe(res);
      upstreamRes.on('end', resolve);
      upstreamRes.on('error', reject);
    });
    ignoreStreamErrors(upstream);
    upstream.on('error', reject);
    req.pipe(upstream);
  });
}
function proxyUpgrade(req, socket, head, flags) {
  ignoreStreamErrors(req);
  ignoreStreamErrors(req.socket);
  ignoreStreamErrors(socket);
  const upstream = http.request({
    host: flags.apiHost,
    port: flags.apiPort,
    path: req.url || '/',
    headers: { ...filteredHeaders(req.headers, `${flags.apiHost}:${flags.apiPort}`), connection: 'Upgrade', upgrade: req.headers.upgrade || 'websocket' },
  });
  upstream.on('upgrade', (upstreamRes, upstreamSocket, upstreamHead) => {
    ignoreStreamErrors(upstreamRes);
    ignoreStreamErrors(upstreamSocket);
    const headerLines = [`HTTP/1.1 ${upstreamRes.statusCode || 101} ${upstreamRes.statusMessage || 'Switching Protocols'}`];
    for (const [key, value] of Object.entries(upstreamRes.headers || {})) {
      if (Array.isArray(value)) value.forEach((entry) => headerLines.push(`${key}: ${entry}`));
      else if (value != null) headerLines.push(`${key}: ${value}`);
    }
    socket.write(`${headerLines.join('\r\n')}\r\n\r\n`);
    if (head && head.length) upstreamSocket.write(head);
    if (upstreamHead && upstreamHead.length) socket.write(upstreamHead);
    upstreamSocket.pipe(socket).pipe(upstreamSocket);
  });
  upstream.on('response', (upstreamRes) => {
    ignoreStreamErrors(upstreamRes);
    socket.write(`HTTP/1.1 ${upstreamRes.statusCode || 502} ${upstreamRes.statusMessage || 'Bad Gateway'}\r\nConnection: close\r\n\r\n`);
    upstreamRes.pipe(socket);
  });
  upstream.on('error', () => { try { socket.destroy(); } catch {} });
  upstream.end();
}
function currentGitBranch() {
  try {
    const result = spawnSync('git', ['branch', '--show-current'], { cwd: ROOT, encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'], timeout: 2000 });
    return cleanText(result.stdout || '', 120);
  } catch { return ''; }
}
async function buildCompatStatus(flags) {
  const [versionRow, toolsRow, usageRow, snapshotRow, configRow] = await Promise.allSettled([
    fetchBackendJson(flags, '/api/version', 8000),
    fetchBackendJson(flags, '/api/tools', 8000),
    fetchBackendJson(flags, '/api/usage', 8000),
    fetchBackendJson(flags, '/api/dashboard/snapshot', 12000),
    fetchBackendJson(flags, '/api/config', 8000),
  ]);
  const version = versionRow.status === 'fulfilled' ? versionRow.value : {};
  const tools = toolsRow.status === 'fulfilled' ? toolsRow.value : {};
  const usage = usageRow.status === 'fulfilled' ? usageRow.value : {};
  const snapshot = snapshotRow.status === 'fulfilled' ? snapshotRow.value : {};
  const config = configRow.status === 'fulfilled' ? configRow.value : {};
  const appTurn = snapshot && snapshot.app && snapshot.app.turn && typeof snapshot.app.turn === 'object' ? snapshot.app.turn : {};
  const usageAgents = usage && Array.isArray(usage.agents) ? usage.agents : [];
  return {
    ok: true,
    version: cleanText(version.version || '0.1.0', 120) || '0.1.0',
    agent_count: parsePositiveInt(usageAgents.length, 0, 0, 100000),
    connected: true,
    uptime_sec: 0,
    uptime_seconds: 0,
    ws: true,
    default_provider: cleanText(config.provider || appTurn.provider || '', 80) || 'unknown',
    default_model: cleanText(config.model || appTurn.model || '', 160) || 'gpt-5',
    git_branch: currentGitBranch(),
    api_listen: `${flags.host}:${flags.port}`,
    listen: `${flags.host}:${flags.port}`,
    home_dir: ROOT,
    workspace_dir: ROOT,
    log_level: cleanText(process.env.RUST_LOG || process.env.LOG_LEVEL || 'info', 32) || 'info',
    network_enabled: true,
    runtime_sync: tools && tools.runtime_sync && typeof tools.runtime_sync === 'object' ? tools.runtime_sync : null,
  };
}
async function buildCompatConfig(flags) {
  const config = await fetchBackendJson(flags, '/api/config', 12000).catch(() => ({}));
  const snapshot = await fetchBackendJson(flags, '/api/dashboard/snapshot', 12000).catch(() => ({}));
  const appTurn = snapshot && snapshot.app && snapshot.app.turn && typeof snapshot.app.turn === 'object' ? snapshot.app.turn : {};
  return {
    ok: true,
    api_key: cleanText(config.api_key || '', 20) || 'set',
    provider: cleanText(config.provider || appTurn.provider || '', 80) || 'openai',
    model: cleanText(config.model || appTurn.model || '', 160) || 'gpt-5',
    cli_mode: cleanText(config.cli_mode || '', 20) || 'ops',
  };
}
async function runServe(flags) {
  ensureSvelteDashboardUiAvailable();
  const backend = await ensureBackend(flags);
  const status = {
    ok: true,
    type: 'infring_dashboard_server',
    ts: nowIso(),
    url: `http://${flags.host}:${flags.port}/dashboard`,
    host: flags.host,
    port: flags.port,
    refresh_ms: flags.refreshMs,
    team: flags.team,
    authority: 'svelte_ui_over_rust_core_api',
    ui_mode: 'svelte',
    backend_url: backendBase(flags),
    backend_reused: backend.reused,
    status_path: path.relative(ROOT, STATUS_PATH),
  };
  const server = http.createServer(async (req, res) => {
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    try {
      if (req.method === 'GET' && (pathname === '/' || pathname === '/dashboard')) {
        const svelteAsset = readSvelteDashboardAsset(pathname);
        if (!svelteAsset) throw new Error('svelte_dashboard_index_missing');
        res.writeHead(200, { 'content-type': svelteAsset.contentType, 'cache-control': 'no-store' });
        res.end(svelteAsset.body);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/status') return void sendJson(res, 200, await buildCompatStatus(flags));
      if (req.method === 'GET' && pathname === '/api/config') return void sendJson(res, 200, await buildCompatConfig(flags));
      if (req.method === 'GET' && pathname === '/api/config/schema') {
        const schema = await fetchBackendJson(flags, '/api/config/schema', 8000).catch(() => ({ ok: true, sections: {} }));
        return void sendJson(res, 200, schema);
      }
      if (req.method === 'GET' && pathname === '/api/auth/check') return void sendJson(res, 200, { ok: true, mode: 'none', authenticated: true, user: 'operator' });
      if (req.method === 'GET') {
        const svelteAsset = readSvelteDashboardAsset(pathname);
        if (svelteAsset) {
          res.writeHead(200, { 'content-type': svelteAsset.contentType, 'cache-control': 'no-store' });
          res.end(svelteAsset.body);
          return;
        }
      }
      if (pathname === '/healthz' || pathname.startsWith('/api/')) return void await proxyToBackend(req, res, flags);
      sendJson(res, 404, { ok: false, type: 'infring_dashboard_not_found', path: pathname });
    } catch (error) {
      sendJson(res, 500, { ok: false, type: 'infring_dashboard_request_error', error: cleanText(error && error.message ? error.message : String(error), 260) });
    }
  });
  server.on('upgrade', (req, socket, head) => {
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    if (!pathname.startsWith('/api/')) { socket.destroy(); return; }
    proxyUpgrade(req, socket, head, flags);
  });
  server.on('clientError', (_error, socket) => {
    try { socket.destroy(); } catch {}
  });
  let cleaned = false;
  const cleanup = () => {
    if (cleaned) return;
    cleaned = true;
    try { server.close(); } catch {}
    if (backend.child && backend.child.exitCode == null) { try { backend.child.kill('SIGTERM'); } catch {} }
  };
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);
  process.on('exit', cleanup);
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(flags.port, flags.host, () => {
      server.off('error', reject);
      ensureDir(STATUS_DIR);
      writeJson(STATUS_PATH, status);
      console.log(JSON.stringify(status, null, 2));
      console.log(`Dashboard listening at ${status.url}`);
      resolve(null);
    });
  });
}
async function run(argv = process.argv.slice(2)) {
  const args = normalizeArgs(argv);
  const flags = parseFlags(args);
  if (flags.mode === 'serve' || flags.mode === 'web') { await runServe(flags); return null; }
  return runProtheusOps(['dashboard-ui', ...args], {
    unknownDomainFallback: true,
    env: {
      PROTHEUS_OPS_USE_PREBUILT: process.env.PROTHEUS_OPS_USE_PREBUILT || '0',
      PROTHEUS_OPS_LOCAL_TIMEOUT_MS: process.env.PROTHEUS_OPS_LOCAL_TIMEOUT_MS || '120000',
    },
  });
}
module.exports = { normalizeArgs, parseFlags, run };
if (require.main === module) {
  process.on('uncaughtException', (error) => {
    if (isTransientSocketError(error)) {
      console.error(cleanText(`dashboard_host_socket:${error.code || 'unknown'}`, 280));
      return;
    }
    console.error(cleanText(error && error.message ? error.message : String(error), 280));
    process.exitCode = 1;
  });
  Promise.resolve(run(process.argv.slice(2)))
    .then((exitCode) => { if (typeof exitCode === 'number') process.exitCode = exitCode; })
    .catch((error) => { console.error(cleanText(error && error.message ? error.message : String(error), 280)); process.exitCode = 1; });
}
