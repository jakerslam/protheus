#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const https = require('https');
const { spawnSync } = require('child_process');
const { sanitizeBridgeArg } = require('../../../client/runtime/lib/runtime_system_entrypoint.ts');

const pkgRoot = path.resolve(__dirname, '..');
const workspaceRoot = path.resolve(pkgRoot, '..', '..');
const pkg = require(path.join(pkgRoot, 'package.json'));
const MAX_DOWNLOAD_BYTES = 64 * 1024 * 1024;
const DOWNLOAD_TIMEOUT_MS = 30000;
const MAX_REDIRECTS = 5;
const ALLOWED_HOSTS = new Set(['github.com', 'objects.githubusercontent.com']);

function exeName() { return process.platform === 'win32' ? 'protheus-ops.exe' : 'protheus-ops'; }
function targetBinaryPath() { return path.join(pkgRoot, 'vendor', exeName()); }
function ensureDir(dirPath) { fs.mkdirSync(dirPath, { recursive: true }); }
function chmodExec(filePath) { if (process.platform !== 'win32') fs.chmodSync(filePath, 0o755); }
function sanitizeText(value, maxLen = 240) { return sanitizeBridgeArg(value, maxLen); }

function platformTriple() {
  const archMap = { x64: 'x86_64', arm64: 'aarch64' };
  const osMap = { darwin: 'apple-darwin', linux: 'unknown-linux-gnu', win32: 'pc-windows-msvc' };
  return (archMap[process.arch] || process.arch) + '-' + (osMap[process.platform] || process.platform);
}

function releaseTag() {
  const override = sanitizeText(process.env.PROTHEUS_NPM_RELEASE_TAG || '', 80);
  if (override) return override.startsWith('v') ? override : `v${override}`;
  return 'v' + pkg.version;
}

function releaseCandidateUrls() {
  const versionTag = releaseTag();
  const triple = platformTriple();
  const base = 'https://github.com/protheuslabs/InfRing/releases/download/' + versionTag;
  const ext = process.platform === 'win32' ? '.exe' : '';
  const stems = ['protheus-ops', 'infring-ops'];
  const urls = [];
  for (const stem of stems) {
    urls.push(base + '/' + stem + '-' + triple + ext);
    urls.push(base + '/' + stem + '-' + triple);
    urls.push(base + '/' + stem + '-' + triple + '.bin');
  }
  const directNames = new Set([exeName(), process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops']);
  for (const name of directNames) {
    urls.push(base + '/' + name);
  }
  return Array.from(new Set(urls));
}

function validateDownloadUrl(rawUrl) {
  try {
    const parsed = new URL(String(rawUrl || ''));
    if (parsed.protocol !== 'https:') return null;
    if (!ALLOWED_HOSTS.has(parsed.hostname)) return null;
    return parsed.toString();
  } catch {
    return null;
  }
}

function download(url, outPath, redirects = 0) {
  return new Promise((resolve, reject) => {
    if (redirects > MAX_REDIRECTS) return reject(new Error('too_many_redirects'));
    const safeUrl = validateDownloadUrl(url);
    if (!safeUrl) return reject(new Error('invalid_download_url'));

    const req = https.get(safeUrl, { timeout: DOWNLOAD_TIMEOUT_MS }, (res) => {
      if (res.statusCode >= 300 && res.statusCode < 400 && res.headers.location) {
        res.resume();
        return download(res.headers.location, outPath, redirects + 1).then(resolve).catch(reject);
      }
      if (res.statusCode !== 200) {
        res.resume();
        return reject(new Error('http_' + res.statusCode));
      }
      const declaredLength = Number(res.headers['content-length'] || 0);
      if (Number.isFinite(declaredLength) && declaredLength > MAX_DOWNLOAD_BYTES) {
        res.resume();
        return reject(new Error('download_too_large'));
      }

      const file = fs.createWriteStream(outPath);
      let total = 0;
      res.on('data', (chunk) => {
        total += Buffer.byteLength(chunk);
        if (total > MAX_DOWNLOAD_BYTES) req.destroy(new Error('download_too_large'));
      });
      res.pipe(file);
      file.on('finish', () => file.close(() => resolve(true)));
      file.on('error', (err) => {
        fs.rmSync(outPath, { force: true });
        reject(err);
      });
    });

    req.on('timeout', () => req.destroy(new Error('download_timeout')));
    req.on('error', (err) => {
      fs.rmSync(outPath, { force: true });
      reject(err);
    });
  });
}

async function tryDownload(outPath) {
  for (const url of releaseCandidateUrls()) {
    try {
      await download(url, outPath);
      chmodExec(outPath);
      process.stdout.write('[protheus npm] downloaded prebuilt binary: ' + url + '\n');
      return true;
    } catch {}
  }
  return false;
}

function tryBuildLocal(outPath) {
  const manifestPath = path.join(workspaceRoot, 'core', 'layer0', 'ops', 'Cargo.toml');
  if (!fs.existsSync(manifestPath)) return false;
  const build = spawnSync('cargo', ['build', '--release', '--manifest-path', manifestPath, '--bin', 'protheus-ops'], { cwd: workspaceRoot, stdio: 'inherit' });
  if (build.status !== 0) return false;
  const candidates = [
    path.join(workspaceRoot, 'target', 'release', exeName()),
    path.join(workspaceRoot, 'target', 'release', process.platform === 'win32' ? 'infring-ops.exe' : 'infring-ops')
  ];
  for (const built of candidates) {
    if (!fs.existsSync(built)) continue;
    fs.copyFileSync(built, outPath);
    chmodExec(outPath);
    process.stdout.write('[protheus npm] built local binary via cargo\n');
    return true;
  }
  return false;
}

async function main() {
  ensureDir(path.join(pkgRoot, 'vendor'));
  const outPath = targetBinaryPath();
  const forceInstall = String(process.env.PROTHEUS_NPM_FORCE_INSTALL || '').trim() === '1';
  if (fs.existsSync(outPath) && !forceInstall) {
    chmodExec(outPath);
    process.stdout.write('[protheus npm] binary already present\n');
    return;
  }

  const skipDownload = String(process.env.PROTHEUS_NPM_SKIP_DOWNLOAD || '').trim() === '1';
  if (!skipDownload) {
    const downloaded = await tryDownload(outPath);
    if (downloaded) return;
  } else {
    process.stdout.write('[protheus npm] skipping release download (PROTHEUS_NPM_SKIP_DOWNLOAD=1)\n');
  }

  if (tryBuildLocal(outPath)) return;

  process.stderr.write('[protheus npm] failed to provision binary (release download unavailable and local cargo build failed)\n');
  process.exit(1);
}

main().catch((err) => {
  process.stderr.write('[protheus npm] install failed: ' + sanitizeText(err && err.message ? err.message : err, 240) + '\n');
  process.exit(1);
});
