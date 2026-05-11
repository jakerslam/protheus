#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import http from 'node:http';
import path from 'node:path';
import { buildBrowserShellV2App } from './browser_shell_v2_build.ts';

const DEFAULT_ARTIFACT_DIR = 'core/local/artifacts/browser_shell_v2_app';
const DEFAULT_HOST = '127.0.0.1';
const DEFAULT_PORT = 5273;

type BrowserShellV2Server = {
  close: () => Promise<void>;
  host: string;
  port: number;
  url: string;
};

function clean(value: unknown, max = 1000): string {
  return String(value == null ? '' : value).trim().slice(0, max);
}

function readFlag(argv: string[], name: string, fallback = ''): string {
  const prefix = `--${name}=`;
  for (let index = 0; index < argv.length; index += 1) {
    const token = clean(argv[index], 1200);
    if (token === `--${name}`) return clean(argv[index + 1], 1200);
    if (token.startsWith(prefix)) return clean(token.slice(prefix.length), 1200);
  }
  return fallback;
}

function parseBool(value: string, fallback = false): boolean {
  const normalized = clean(value, 32).toLowerCase();
  if (!normalized) return fallback;
  return ['1', 'true', 'yes', 'on'].includes(normalized);
}

function write(filePath: string, body: string): void {
  const abs = path.resolve(process.cwd(), filePath);
  fs.mkdirSync(path.dirname(abs), { recursive: true });
  fs.writeFileSync(abs, body, 'utf8');
}

function contentType(filePath: string): string {
  if (filePath.endsWith('.html')) return 'text/html; charset=utf-8';
  if (filePath.endsWith('.css')) return 'text/css; charset=utf-8';
  if (filePath.endsWith('.js')) return 'text/javascript; charset=utf-8';
  if (filePath.endsWith('.json')) return 'application/json; charset=utf-8';
  return 'application/octet-stream';
}

function waitForever(): Promise<void> {
  return new Promise(() => {});
}

function safeArtifactPath(artifactDir: string, requestUrl = '/'): string {
  const url = new URL(requestUrl, 'http://browser-shell-v2.local');
  const pathname = decodeURIComponent(url.pathname === '/' ? '/index.html' : url.pathname);
  const root = path.resolve(process.cwd(), artifactDir);
  const target = path.resolve(root, `.${pathname}`);
  if (!target.startsWith(root)) return path.join(root, 'index.html');
  return target;
}

export async function startBrowserShellV2Server(options: {
  artifactDir?: string;
  host?: string;
  port?: number;
  buildFirst?: boolean;
} = {}): Promise<BrowserShellV2Server> {
  const artifactDir = options.artifactDir || DEFAULT_ARTIFACT_DIR;
  const host = options.host || DEFAULT_HOST;
  const port = Number.isFinite(options.port) ? Number(options.port) : DEFAULT_PORT;
  if (options.buildFirst !== false) buildBrowserShellV2App(artifactDir);
  const server = http.createServer((request, response) => {
    const filePath = safeArtifactPath(artifactDir, request.url || '/');
    if (!fs.existsSync(filePath) || !fs.statSync(filePath).isFile()) {
      response.writeHead(404, { 'content-type': 'application/json; charset=utf-8' });
      response.end(JSON.stringify({ ok: false, error: 'browser_shell_v2_asset_not_found' }));
      return;
    }
    response.writeHead(200, {
      'cache-control': 'no-store',
      'content-type': contentType(filePath),
    });
    response.end(fs.readFileSync(filePath));
  });
  await new Promise<void>((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, host, () => resolve());
  });
  const address = server.address();
  const actualPort = typeof address === 'object' && address ? address.port : port;
  return {
    close: () => new Promise((resolve, reject) => server.close((error) => (error ? reject(error) : resolve()))),
    host,
    port: actualPort,
    url: `http://${host}:${actualPort}/`,
  };
}

async function smoke(argv: string[]): Promise<Record<string, unknown>> {
  const artifactDir = readFlag(argv, 'artifact-dir', DEFAULT_ARTIFACT_DIR);
  const server = await startBrowserShellV2Server({ artifactDir, port: 0 });
  try {
    const [indexResponse, runtimeResponse] = await Promise.all([
      fetch(server.url),
      fetch(`${server.url}browser_shell_v2_app.js`),
    ]);
    const indexText = await indexResponse.text();
    const runtimeText = await runtimeResponse.text();
    const ok = indexResponse.ok
      && runtimeResponse.ok
      && indexText.includes('browser-shell-v2-root')
      && runtimeText.includes('/api/shell-socket/runtime-status')
      && runtimeText.includes('/api/shell-socket/input');
    return {
      ok,
      type: 'browser_shell_v2_serve_smoke',
      url: server.url,
      artifact_dir: artifactDir,
      index_status: indexResponse.status,
      runtime_status: runtimeResponse.status,
      socket_routes_present: runtimeText.includes('/api/shell-socket/runtime-status') && runtimeText.includes('/api/shell-socket/input'),
    };
  } finally {
    await server.close();
  }
}

async function main(): Promise<void> {
  const argv = process.argv.slice(2);
  if (parseBool(readFlag(argv, 'serve-smoke'), false)) {
    const result = await smoke(argv);
    const outJson = readFlag(argv, 'out-json', 'core/local/artifacts/browser_shell_v2_serve_smoke_current.json');
    const outMarkdown = readFlag(argv, 'out-markdown', 'local/workspace/reports/BROWSER_SHELL_V2_SERVE_SMOKE_CURRENT.md');
    write(outJson, `${JSON.stringify(result, null, 2)}\n`);
    write(outMarkdown, `# Browser Shell V2 Serve Smoke\n\nok: \`${result.ok}\`\nurl: \`${result.url}\`\n`);
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
    process.exitCode = result.ok ? 0 : 1;
    return;
  }
  if (parseBool(readFlag(argv, 'serve'), false)) {
    const artifactDir = readFlag(argv, 'artifact-dir', DEFAULT_ARTIFACT_DIR);
    const host = readFlag(argv, 'host', DEFAULT_HOST);
    const port = Number(readFlag(argv, 'port', String(DEFAULT_PORT)));
    const server = await startBrowserShellV2Server({ artifactDir, host, port });
    process.stdout.write(`[browser-shell-v2] serving ${server.url}?gateway=http://127.0.0.1:5173\n`);
    const shutdown = async () => {
      await server.close();
      process.exit(0);
    };
    process.on('SIGINT', () => { void shutdown(); });
    process.on('SIGTERM', () => { void shutdown(); });
    await waitForever();
  }
}

if (process.argv[1]?.endsWith('browser_shell_v2_server.ts')) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
