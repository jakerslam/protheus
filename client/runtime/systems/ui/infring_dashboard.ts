#!/usr/bin/env tsx
// Thin dashboard UI host: serves the Infring browser UI over the Rust API lane.

const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const { WebSocketServer } = require('ws');
const { spawn } = require('node:child_process');
const { ROOT, resolveBinary, runProtheusOps } = require('../ops/run_protheus_ops.ts');
const { buildPrimaryDashboardHtml, hasPrimaryDashboardUi, readPrimaryDashboardAsset } = require('./dashboard_asset_router.ts');

const DASHBOARD_DIR = __dirname;
const STATIC_DIR = path.resolve(DASHBOARD_DIR, 'openclaw_static');
const FORBIDDEN_ALT_DASHBOARD_DIRS = [
  path.resolve(DASHBOARD_DIR, 'dashboard_sveltekit'),
  path.resolve(DASHBOARD_DIR, 'openfang_static'),
  path.resolve(DASHBOARD_DIR, 'legacy_dashboard'),
];
const SIBLING_ALT_DASHBOARD_PATTERN = /(dashboard|legacy|openfang|svelte)/i;
const STATUS_DIR = path.resolve(ROOT, 'client/runtime/local/state/ui/infring_dashboard');
const STATUS_PATH = path.resolve(STATUS_DIR, 'server_status.json');
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 4173;
const DEFAULT_TEAM = 'ops';
const DEFAULT_REFRESH_MS = 2000;
const DEFAULT_BACKEND_READY_TIMEOUT_MS = 120000;
const BACKEND_PORT_OFFSET = 1000;
const HOP_BY_HOP = new Set(['connection', 'host', 'keep-alive', 'proxy-authenticate', 'proxy-authorization', 'te', 'trailers', 'transfer-encoding', 'upgrade']);

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
function discoverSiblingAltDashboardSurfaces() {
  const out = [];
  let rows = [];
  try { rows = fs.readdirSync(DASHBOARD_DIR, { withFileTypes: true }); } catch { return out; }
  for (const entry of rows) {
    if (!entry || typeof entry.isDirectory !== 'function' || !entry.isDirectory()) continue;
    const dirPath = path.resolve(DASHBOARD_DIR, String(entry.name || ''));
    if (!dirPath || dirPath === STATIC_DIR) continue;
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
function spawnBackend(flags) {
  const laneArgs = ['dashboard-ui', 'serve', `--host=${flags.apiHost}`, `--port=${flags.apiPort}`, `--team=${flags.team}`, `--refresh-ms=${flags.refreshMs}`];
  const env = {
    ...process.env,
    PROTHEUS_ROOT: ROOT,
    PROTHEUS_OPS_ALLOW_STALE: process.env.PROTHEUS_OPS_ALLOW_STALE || '1',
    PROTHEUS_NPM_ALLOW_STALE: process.env.PROTHEUS_NPM_ALLOW_STALE || '1',
  };
  const explicitBin = cleanText(env.PROTHEUS_NPM_BINARY || '', 600);
  const bin = explicitBin || resolveBinary({ env });
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
function createAgentWsBridge(flags) {
  const wss = new WebSocketServer({ noServer: true, clientTracking: false, perMessageDeflate: false });
  const route = /^\/api\/agents\/([^/]+)\/ws$/;
  const enc = (agentId) => encodeURIComponent(String(agentId || '').trim());
  const send = (ws, payload) => {
    try { if (ws && ws.readyState === 1) ws.send(JSON.stringify(payload)); } catch {}
  };
  const parseJson = (raw) => { try { return JSON.parse(raw); } catch { return null; } };
  const toNum = (value, fallback = 0) => Number.isFinite(Number(value)) ? Number(value) : fallback;
  const sendContext = async (ws, agentId) => {
    const agent = await fetchBackendJson(flags, `/api/agents/${enc(agentId)}`, 8000).catch(() => ({}));
    const contextWindow = toNum(agent.context_window || agent.context_window_tokens || 0, 0);
    send(ws, { type: 'context_state', agent_id: agentId, context_tokens: 0, context_window: contextWindow, context_ratio: 0, context_pressure: contextWindow > 0 ? 'normal' : '' });
    return agent;
  };
  wss.on('connection', (ws, _req, agentId) => {
    const targetAgent = cleanText(agentId || '', 180);
    let agentName = '';
    let chain = Promise.resolve();
    chain = chain.then(async () => {
      const agent = await sendContext(ws, targetAgent);
      agentName = cleanText(agent.name || '', 120);
      send(ws, { type: 'connected', agent_id: targetAgent, agent_name: agentName || '' });
    }).catch((error) => send(ws, { type: 'error', content: cleanText(error && error.message ? error.message : 'ws_connect_failed', 260), agent_id: targetAgent }));
    ws.on('message', (chunk) => {
      const raw = Buffer.isBuffer(chunk) ? chunk.toString('utf8') : String(chunk || '');
      chain = chain.then(async () => {
        const payload = parseJson(raw);
        if (!payload || typeof payload !== 'object') return;
        const msgType = cleanText(payload.type || '', 40).toLowerCase();
        if (!msgType || msgType === 'ping') { send(ws, { type: 'pong' }); return; }
        if (msgType === 'message') {
          const content = String(payload.content == null ? '' : payload.content).slice(0, 12000);
          if (!content.trim()) { send(ws, { type: 'error', content: 'message_required', agent_id: targetAgent }); return; }
          send(ws, { type: 'typing', state: 'start', agent_id: targetAgent });
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/message`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ message: content, attachments: Array.isArray(payload.attachments) ? payload.attachments : [] }),
          }, 180000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, {
            type: 'response',
            agent_id: targetAgent,
            agent_name: agentName || cleanText(out.agent_name || '', 120) || '',
            content: String(out.response || out.content || ''),
            input_tokens: toNum(out.input_tokens || 0, 0),
            output_tokens: toNum(out.output_tokens || 0, 0),
            cost_usd: toNum(out.cost_usd || 0, 0),
            iterations: toNum(out.iterations || 1, 1),
            duration_ms: toNum(out.duration_ms || out.latency_ms || 0, 0),
            context_tokens: toNum(out.context_tokens || out.context_used_tokens || out.context_total_tokens || 0, 0),
            context_window: toNum(out.context_window || out.context_window_tokens || 0, 0),
            context_ratio: toNum(out.context_ratio || 0, 0),
            context_pressure: cleanText(out.context_pressure || '', 32),
            auto_route: out.auto_route || null,
          });
          return;
        }
        if (msgType === 'terminal') {
          const command = String(payload.command == null ? '' : payload.command).slice(0, 16000);
          if (!command.trim()) { send(ws, { type: 'terminal_error', agent_id: targetAgent, message: 'command_required' }); return; }
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/terminal`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, cwd: cleanText(payload.cwd || '', 4000) }),
          }, 120000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'terminal_error', agent_id: targetAgent, message: cleanText(out.error || out.message || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, { type: 'terminal_output', agent_id: targetAgent, stdout: String(out.stdout || ''), stderr: String(out.stderr || ''), exit_code: toNum(out.exit_code || 0, 0), duration_ms: toNum(out.duration_ms || 0, 0), cwd: cleanText(out.cwd || '', 4000) });
          return;
        }
        if (msgType === 'command') {
          const command = cleanText(payload.command || '', 80).toLowerCase();
          const res = await fetchBackend(flags, `/api/agents/${enc(targetAgent)}/command`, {
            method: 'POST',
            headers: { 'content-type': 'application/json' },
            body: JSON.stringify({ command, silent: !!payload.silent }),
          }, 12000);
          const out = await res.json().catch(() => ({}));
          if (!res.ok || out.ok === false) {
            send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(out.error || `backend_http_${res.status}`, 260) });
            return;
          }
          send(ws, {
            type: 'command_result',
            silent: !!payload.silent,
            agent_id: targetAgent,
            command: cleanText(out.command || command || 'unknown', 80),
            message: cleanText(out.message || `Command '${command || 'unknown'}' acknowledged.`, 320),
            runtime_sync: out.runtime_sync || null,
            context_window: toNum(out.context_window || 0, 0),
          });
          return;
        }
      }).catch((error) => send(ws, { type: 'error', agent_id: targetAgent, content: cleanText(error && error.message ? error.message : 'ws_bridge_failed', 260) }));
    });
    ws.on('error', () => {});
  });
  return {
    tryHandle(req, socket, head) {
      const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
      const match = pathname.match(route);
      if (!match) return false;
      const agentId = cleanText(decodeURIComponent(match[1] || ''), 180);
      if (!agentId) { try { socket.destroy(); } catch {} return true; }
      wss.handleUpgrade(req, socket, head, (ws) => wss.emit('connection', ws, req, agentId));
      return true;
    },
  };
}
async function runServe(flags) {
  assertDashboardSurfaceLocked();
  let dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR);
  if (!dashboardHtml.trim()) throw new Error('primary_dashboard_html_empty');
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
    authority: 'primary_dashboard_ui_over_rust_core_api',
    backend_url: backendBase(flags),
    backend_reused: backend.reused,
    status_path: path.relative(ROOT, STATUS_PATH),
  };
  const wsBridge = createAgentWsBridge(flags);
  const server = http.createServer(async (req, res) => {
    const pathname = new URL(req.url || '/', `http://${flags.host}:${flags.port}`).pathname;
    try {
      if (req.method === 'GET' && pathname === '/dashboard-shell') {
        dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(dashboardHtml);
        return;
      }
      if (req.method === 'GET' && (pathname === '/' || pathname === '/dashboard')) {
        dashboardHtml = buildPrimaryDashboardHtml(STATIC_DIR) || dashboardHtml;
        res.writeHead(200, { 'content-type': 'text/html; charset=utf-8', 'cache-control': 'no-store' });
        res.end(dashboardHtml);
        return;
      }
      if (req.method === 'GET' && pathname === '/api/status') {
        const status = await fetchBackendJson(flags, '/api/status', 8000).catch(() => ({ ok: false, error: 'status_unavailable' }));
        return void sendJson(res, 200, status);
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
      if (req.method === 'GET') {
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
      sendJson(res, 500, { ok: false, type: 'infring_dashboard_request_error', error: cleanText(error && error.message ? error.message : String(error), 260) });
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
