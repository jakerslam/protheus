#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { spawn } = require('node:child_process');
const {
  ROOT,
  invokeProtheusOpsViaBridge,
  resolveBinary,
  runProtheusOps,
} = require('./run_protheus_ops.ts');
const { buildPrimaryDashboardHtml, hasPrimaryDashboardUi, readBuildVersionInfo, readPrimaryDashboardAsset } = require('./dashboard_asset_router.ts');
const { createAgentWsBridge } = require('./agent_ws_bridge.ts');

const DASHBOARD_DIR = path.resolve(ROOT, 'client/runtime/systems/ui');
const CANONICAL_STATIC_DIR = path.resolve(DASHBOARD_DIR, 'infring_static');
const SVELTEKIT_MODULE_DIR = path.resolve(DASHBOARD_DIR, 'dashboard_sveltekit');
const SVELTEKIT_BUILD_DIR = path.resolve(SVELTEKIT_MODULE_DIR, 'build');
const SVELTEKIT_INDEX_PATH = path.resolve(SVELTEKIT_BUILD_DIR, 'index.html');
const STATIC_DIR = CANONICAL_STATIC_DIR;
const FORBIDDEN_ALT_DASHBOARD_DIRS = [
  path.resolve(DASHBOARD_DIR, 'legacy_dashboard'),
  path.resolve(DASHBOARD_DIR, 'reference_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'control_runtime_dashboard'),
  path.resolve(DASHBOARD_DIR, 'dashboard_legacy'),
  path.resolve(DASHBOARD_DIR, 'deprecated_dashboard'),
];
const SIBLING_ALT_DASHBOARD_PATTERN = /(legacy|reference_runtime|control_runtime|deprecated)/i;
const STATUS_DIR = path.resolve(ROOT, 'client/runtime/local/state/ui/infring_dashboard');
const STATUS_PATH = path.resolve(STATUS_DIR, 'server_status.json');
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 2000;
const DEFAULT_BACKEND_READY_TIMEOUT_MS = 120000;
const BACKEND_PORT_OFFSET = 1000;
const HOP_BY_HOP = new Set(['connection', 'host', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade']);

function hasSvelteKitBuild() {
  try {
    return fs.statSync(SVELTEKIT_INDEX_PATH).isFile();
  } catch {
    return false;
  }
}
function svelteKitContentType(filePath) {
  const ext = path.extname(String(filePath || '')).toLowerCase();
  if (ext === '.html') return 'text/html; charset=utf-8';
  if (ext === '.js' || ext === '.mjs') return 'text/javascript; charset=utf-8';
  if (ext === '.css') return 'text/css; charset=utf-8';
  if (ext === '.svg') return 'image/svg+xml; charset=utf-8';
  if (ext === '.json' || ext === '.map') return 'application/json; charset=utf-8';
  if (ext === '.txt') return 'text/plain; charset=utf-8';
  if (ext === '.ico') return 'image/x-icon';
  if (ext === '.png') return 'image/png';
  if (ext === '.jpg' || ext === '.jpeg') return 'image/jpeg';
  if (ext === '.webp') return 'image/webp';
  if (ext === '.woff') return 'font/woff';
  if (ext === '.woff2') return 'font/woff2';
  return 'application/octet-stream';
}
function readSvelteKitAsset(pathname) {
  if (!hasSvelteKitBuild()) return null;
  const rawPath = String(pathname || '/');
  const fromDashboardPrefix = rawPath.startsWith('/dashboard/') ? rawPath.slice('/dashboard'.length) : rawPath;
  const normalized = rawPath === '/' || rawPath === '/dashboard' || rawPath === '/dashboard/' ? '/index.html' : (fromDashboardPrefix || '/');
  const relPath = String(normalized || '/').replace(/^\/+/, '');
  const candidate = path.resolve(SVELTEKIT_BUILD_DIR, relPath);
  if (candidate.startsWith(SVELTEKIT_BUILD_DIR)) {
    try {
      if (fs.statSync(candidate).isFile()) {
        return { body: fs.readFileSync(candidate), contentType: svelteKitContentType(candidate) };
      }
    } catch {}
  }
  if (rawPath === '/' || rawPath === '/dashboard' || rawPath.startsWith('/dashboard/')) {
    return { body: fs.readFileSync(SVELTEKIT_INDEX_PATH), contentType: 'text/html; charset=utf-8' };
  }
  return null;
}
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
  const out = {
    mode: 'serve',
    host: DEFAULT_HOST,
    port: DEFAULT_PORT,
    team: DEFAULT_TEAM,
    refreshMs: DEFAULT_REFRESH_MS,
    pretty: true,
    apiHost: '',
    apiPort: 0,
    apiReadyTimeoutMs: DEFAULT_BACKEND_READY_TIMEOUT_MS,
    uiMode: cleanText(process.env.INFRING_DASHBOARD_UI || 'sveltekit', 24).toLowerCase(),
  };
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
    else if (value.startsWith('--ui=')) out.uiMode = cleanText(value.slice(5), 24).toLowerCase();
    else if (value === '--pretty=0' || value === '--pretty=false') out.pretty = false;
  }
  if (out.uiMode !== 'sveltekit') out.uiMode = 'classic';
  out.apiHost = out.apiHost || out.host;
  out.apiPort = out.apiPort || defaultApiPort(out.port);
  if (out.apiPort === out.port) out.apiPort = defaultApiPort(out.port + 1);
  return out;
}
function ensureDir(dirPath) { fs.mkdirSync(dirPath, { recursive: true }); }
function writeJson(filePath, value) { ensureDir(path.dirname(filePath)); fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8'); }
function discoverSiblingAltDashboardSurfaces() {
  const out = [];
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return out; }
  for (const entry of rows) {
    if (!entry || typeof entry.isDirectory !== 'function' || !entry.isDirectory()) continue;
    const dirPath = path.resolve(DASHBOARD_DIR, String(entry.name || ''));
    if (!dirPath || dirPath === STATIC_DIR) continue;
    if (dirPath === SVELTEKIT_MODULE_DIR) continue;
    const dirName = path.basename(dirPath);
    const hasInlineDashboardRoot = hasPrimaryDashboardUi(dirPath);
    const hasBuildIndex = fs.existsSync(path.resolve(dirPath, 'build', 'index.html'));
    const hasIndexHtml = fs.existsSync(path.resolve(dirPath, 'index.html'));
    if (SIBLING_ALT_DASHBOARD_PATTERN.test(dirName) || hasInlineDashboardRoot || hasBuildIndex || hasIndexHtml) out.push(dirPath);
  }
  return out;
}
function assertNoAlternateDashboardSurfaces() {
  const found = new Set();
  FORBIDDEN_ALT_DASHBOARD_DIRS.filter((dirPath) => fs.existsSync(dirPath)).forEach((dirPath) => found.add(dirPath));
  discoverSiblingAltDashboardSurfaces().forEach((dirPath) => found.add(dirPath));
  if (found.size === 0) return;
  const labels = Array.from(found).map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`forbidden_dashboard_surface_present:${labels}`);
}
function assertSingleDashboardRoot() {
  if (!hasPrimaryDashboardUi(STATIC_DIR)) throw new Error('primary_dashboard_ui_missing');
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return; }
  const duplicateRoots = rows
    .filter((entry) => entry && typeof entry.isDirectory === 'function' && entry.isDirectory())
    .map((entry) => path.resolve(DASHBOARD_DIR, String(entry.name || '')))
    .filter((dirPath) => dirPath !== STATIC_DIR && hasPrimaryDashboardUi(dirPath));
  if (!duplicateRoots.length) return;
  const labels = duplicateRoots.map((dirPath) => path.basename(dirPath)).sort((a, b) => a.localeCompare(b, 'en')).join(',');
  throw new Error(`multiple_dashboard_roots_detected:${labels}`);
}
function assertDashboardSurfaceLocked() {
  assertNoAlternateDashboardSurfaces();
  assertSingleDashboardRoot();
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
async function statusPayloadWithBootStage(flags) {
  const startedAt = Date.now();
  const healthOk = await backendHealth(flags, 1200);
  if (!healthOk) {
    return {
      ok: false,
      error: 'backend_unreachable',
      connected: false,
      connection_state: 'disconnected',
      boot_stage: 'backend_unreachable',
      backend_health_ok: false,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
  try {
    const status = await fetchBackendJson(flags, '/api/status', 1800);
    const base = (status && typeof status === 'object') ? status : {};
    const connected = base.connected !== false;
    const degraded = !!base.degraded || base.ok === false;
    const out = {
      ...base,
      ok: connected,
      connected,
      degraded,
      connection_state: connected ? 'connected' : 'disconnected',
      boot_stage: cleanText(base.boot_stage || base.last_stage || (degraded ? 'status_degraded' : 'ready'), 60),
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
    };
    if (!out.error && degraded) out.error = 'status_degraded';
    return out;
  } catch {
    return {
      ok: true,
      degraded: true,
      warning: 'status_unavailable',
      connected: true,
      connection_state: 'connected',
      boot_stage: 'backend_ready_status_probe_timeout',
      backend_health_ok: true,
      status_latency_ms: Date.now() - startedAt,
      retry_after_ms: 1000,
    };
  }
}
function spawnBackend(flags) {
  const laneArgs = ['dashboard-ui', 'serve', `--host=${flags.apiHost}`, `--port=${flags.apiPort}`, `--team=${flags.team}`, `--refresh-ms=${flags.refreshMs}`];
  const env = {
    ...process.env,
    PROTHEUS_ROOT: ROOT,
    PROTHEUS_OPS_ALLOW_STALE: process.env.PROTHEUS_OPS_ALLOW_STALE || '1',
    PROTHEUS_NPM_ALLOW_STALE: process.env.PROTHEUS_NPM_ALLOW_STALE || '1',
  };
  const bin = resolveBinary({ env });
  if (!bin) throw new Error('dashboard_backend_binary_missing');
  const child = spawn(bin, laneArgs, { cwd: ROOT, env, stdio: ['ignore', 'pipe', 'pipe'] });
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
function parseLastJson(stdout) {
  const lines = String(stdout || '')
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    const line = lines[i];
    if (!line.startsWith('{')) continue;
    try {
      return JSON.parse(line);
    } catch {}
  }
  return null;
}
function readJsonBody(req, maxBytes = 65536) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let total = 0;
    ignoreStreamErrors(req);
    req.on('data', (chunk) => {
      const next = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
      total += next.length;
      if (total > maxBytes) {
        reject(new Error('request_body_too_large'));
        return;
      }
      chunks.push(next);
    });
    req.on('end', () => {
      if (!chunks.length) {
        resolve({});
        return;
      }
      try {
        resolve(JSON.parse(Buffer.concat(chunks).toString('utf8') || '{}'));
      } catch {
        reject(new Error('request_body_invalid_json'));
      }
    });
    req.on('error', reject);
  });
}
function currentDashboardBuildInfo() {
  return readBuildVersionInfo(STATIC_DIR);
}
function mergeDashboardVersionPayload(payload) {
  const base = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  const build = currentDashboardBuildInfo();
  const version = cleanText(build && build.version, 120) || '0.0.0';
  const tag = cleanText(build && build.tag, 120) || `v${version}`;
  const source = cleanText(build && build.source, 80) || 'fallback_default';
  return {
    ...base,
    ok: base.ok !== false,
    version,
    tag,
    version_tag: tag,
    source,
    version_source: source,
    platform: base.platform || process.platform,
    arch: base.arch || process.arch,
  };
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
function dashboardSystemActionArgs(action, payload = {}) {
  const normalized = cleanText(action, 40).toLowerCase();
  const body = (payload && typeof payload === 'object' && !Array.isArray(payload)) ? payload : {};
  if (normalized === 'restart') return ['restart', '--json'];
  if (normalized === 'shutdown') return ['stop', '--json'];
  if (normalized === 'update') {
    const args = ['update', '--json'];
    if (body.force === true) args.push('--force');
    if (body.apply !== false) args.push('--apply');
    return args;
  }
  throw new Error(`unknown_dashboard_system_action:${normalized}`);
}
function dashboardSystemActionEnv() {
  return {
    ...process.env,
    PROTHEUS_ROOT: ROOT,
    PROTHEUS_OPS_ALLOW_STALE: process.env.PROTHEUS_OPS_ALLOW_STALE || '1',
    PROTHEUS_NPM_ALLOW_STALE: process.env.PROTHEUS_NPM_ALLOW_STALE || '1',
  };
}
function runDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const run =
    invokeProtheusOpsViaBridge(args, {
      allowProcessFallback: false,
      unknownDomainFallback: false,
    }) || {
      status: 1,
      stdout: '',
      stderr: 'resident_ipc_bridge_unavailable',
      payload: null,
    };
  const status = Number.isFinite(Number(run.status)) ? Number(run.status) : 1;
  const receipt = (run && run.payload && typeof run.payload === 'object') ? run.payload : parseLastJson(run.stdout);
  const ok = status === 0 && (!receipt || receipt.ok !== false);
  const error = ok
    ? ''
    : cleanText(
        (receipt && receipt.error) || run.stderr || run.stdout || `${cleanText(action, 40).toLowerCase()}_failed`,
        260,
      );
  return {
    ok,
    type: 'dashboard_system_action',
    action: cleanText(action, 40).toLowerCase(),
    command: args[0],
    args: args.slice(1),
    exit_code: status,
    payload: receipt || null,
    error,
  };
}
function dispatchDashboardSystemAction(action, payload = {}) {
  const args = dashboardSystemActionArgs(action, payload);
  const env = dashboardSystemActionEnv();
  const bin = resolveBinary({ env });
  if (!bin) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: '',
      args: args.slice(1),
      error: 'dashboard_backend_binary_missing',
    };
  }
  try {
    const child = spawn(bin, args, {
      cwd: ROOT,
      env,
      detached: true,
      stdio: 'ignore',
    });
    if (child && typeof child.unref === 'function') child.unref();
    return {
      ok: true,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(bin),
      args: args.slice(1),
      dispatch_mode: 'detached_subprocess',
      pid: Number(child && child.pid) || 0,
      payload: null,
      error: '',
    };
  } catch (error) {
    return {
      ok: false,
      type: 'dashboard_system_action',
      action: cleanText(action, 40).toLowerCase(),
      command: path.basename(String(bin || '')),
      args: args.slice(1),
      error: cleanText(error && error.message ? error.message : String(error), 260),
    };
  }
}
function scheduleDashboardHostExit(cleanup, delayMs = 180) {
  const waitMs = parsePositiveInt(delayMs, 180, 80, 5000);
  setTimeout(() => {
    try { cleanup(); } catch {}
    setTimeout(() => {
      try { process.exit(0); } catch {}
    }, 0);
  }, waitMs);
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
async function runServe(flags) {
  assertDashboardSurfaceLocked();
  const svelteKitUiEnabled = flags.uiMode === 'sveltekit' && hasSvelteKitBuild();
  let dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR);
  if (!dashboardHtml.trim()) throw new Error('primary_dashboard_html_empty');
  if (flags.uiMode === 'sveltekit' && !svelteKitUiEnabled) {
    console.warn('dashboard_sveltekit_build_missing_using_primary_static_ui');
  }
  const backend = await ensureBackend(flags);
  const wsBridge = createAgentWsBridge({ flags, cleanText, fetchBackend, fetchBackendJson });
  const status = {
    ok: true,
    type: 'infring_dashboard_server',
    ts: nowIso(),
    url: `http://${flags.host}:${flags.port}/dashboard`,
    host: flags.host,
    port: flags.port,
    refresh_ms: flags.refreshMs,
    team: flags.team,
    authority: 'primary_dashboard_ui_over_rust_core_api',
    dashboard_ui_mode_requested: flags.uiMode,
    dashboard_ui_mode_active: svelteKitUiEnabled ? 'sveltekit' : 'classic',
    backend_url: backendBase(flags),
    backend_reused: backend.reused,
    ws_bridge_enabled: !!wsBridge.ws_enabled,
    ws_bridge_error: cleanText(wsBridge.ws_error || '', 120),
    dashboard_static_dir: path.basename(STATIC_DIR),
    dashboard_sveltekit_module: fs.existsSync(SVELTEKIT_MODULE_DIR),
    status_path: path.relative(ROOT, STATUS_PATH),
  };
  const server = http.createServer(async (req, res) => {
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    try {
      if (req.method === 'GET' && (pathname === '/dashboard-classic' || pathname === '/dashboard-shell')) {
        dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(dashboardHtml);
        return;
      }
      if (req.method === 'GET' && (pathname === '/' || pathname === '/dashboard')) {
        if (svelteKitUiEnabled) {
          if (pathname === '/') {
            res.writeHead(302, { location: '/dashboard', 'cache-control': 'no-store' });
            res.end();
            return;
          }
          const asset = readSvelteKitAsset(pathname);
          if (asset) {
            res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
            res.end(asset.body);
            return;
          }
        } else {
          dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
          res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
          res.end(dashboardHtml);
          return;
        }
      }
      if (req.method === 'GET' && pathname === '/api/status') {
        const status = mergeDashboardVersionPayload(await statusPayloadWithBootStage(flags));
        return void sendJson(res, 200, status);
      }
      if (req.method === 'GET' && pathname === '/api/version') {
        const versionPayload = await fetchBackendJson(flags, '/api/version', 4000).catch(() => ({ ok: true }));
        return void sendJson(res, 200, mergeDashboardVersionPayload(versionPayload));
      }
      if (req.method === 'GET' && pathname === '/api/config') {
        const config = await fetchBackendJson(flags, '/api/config', 8000).catch(() => ({ ok: false, error: 'config_unavailable' }));
        return void sendJson(res, 200, config);
      }
      if (req.method === 'GET' && pathname === '/api/config/schema') {
        const schema = await fetchBackendJson(flags, '/api/config/schema', 8000).catch(() => ({ ok: true, sections: {} }));
        return void sendJson(res, 200, schema);
      }
      if (req.method === 'GET' && pathname === '/api/auth/check') {
        const auth = await fetchBackendJson(flags, '/api/auth/check', 8000).catch(() => ({ ok: true, mode: 'none', authenticated: true, user: 'operator' }));
        return void sendJson(res, 200, auth);
      }
      if (req.method === 'POST' && pathname === '/api/system/restart') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('restart', body);
        return void sendJson(res, result.ok ? 200 : 500, result);
      }
      if (req.method === 'POST' && pathname === '/api/system/update') {
        const body = await readJsonBody(req);
        try {
          const upstream = await fetchBackend(flags, '/api/system/update', {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify(body || {})
          }, body && body.apply === false ? 8000 : 3500);
          const text = await upstream.text();
          let payload = {};
          try {
            payload = text ? JSON.parse(text) : {};
          } catch {
            payload = {};
          }
          return void sendJson(
            res,
            upstream.status || ((payload && payload.ok === false) ? 400 : 200),
            payload && typeof payload === 'object' ? payload : { ok: upstream.ok }
          );
        } catch (_) {
          const result = runDashboardSystemAction('update', body);
          return void sendJson(res, result.ok ? 200 : 500, result);
        }
      }
      if (req.method === 'POST' && pathname === '/api/system/shutdown') {
        const body = await readJsonBody(req);
        const result = dispatchDashboardSystemAction('shutdown', body);
        sendJson(res, result.ok ? 200 : 500, result);
        if (result.ok) {
          scheduleDashboardHostExit(cleanup, body && body.exit_delay_ms);
        }
        return;
      }
      if (req.method === 'GET') {
        if (svelteKitUiEnabled) {
          const svelteAsset = readSvelteKitAsset(pathname);
          if (svelteAsset) {
            res.writeHead(200, { 'content-type': svelteAsset.contentType, 'cache-control': 'no-store' });
            res.end(svelteAsset.body);
            return;
          }
        }
        const asset = readPrimaryDashboardAsset(STATIC_DIR, pathname);
        if (asset) {
          res.writeHead(200, { 'content-type': asset.contentType, 'cache-control': 'no-store' });
          res.end(asset.body);
          return;
        }
      }
      if (pathname === '/healthz' || pathname.startsWith('/api/')) return void await proxyToBackend(req, res, flags);
      sendJson(res, 404, { ok: false, type: 'infring_dashboard_not_found', path: pathname });
    } catch (error) {
      const message = cleanText(error && error.message ? error.message : String(error), 260);
      const statusCode = message === 'request_body_invalid_json' || message === 'request_body_too_large' ? 400 : 500;
      sendJson(res, statusCode, { ok: false, type: 'infring_dashboard_request_error', error: message });
    }
  });
  server.on('upgrade', (req, socket, head) => {
    if (wsBridge.tryHandle(req, socket, head)) return;
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
module.exports = {
  cleanText,
  currentDashboardBuildInfo,
  dashboardSystemActionArgs,
  isTransientSocketError,
  mergeDashboardVersionPayload,
  normalizeArgs,
  parseFlags,
  dispatchDashboardSystemAction,
  run,
  runDashboardSystemAction,
  scheduleDashboardHostExit,
};
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
