#!/usr/bin/env tsx

const childProcess = require('node:child_process');
const fs = require('node:fs');
const path = require('node:path');

const HIGHLIGHT_JS_CDN_URL = 'https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.11.1/highlight.min.js';

const PAGE_SCRIPTS = ['overview', 'chat', 'agents', 'workflows', 'workflow-builder', 'channels', 'eyes', 'skills', 'hands', 'scheduler', 'settings', 'usage', 'sessions', 'logs', 'wizard', 'approvals', 'comms', 'runtime'];
const MIME = {
  '.css': 'text/css; charset=utf-8',
  '.html': 'text/html; charset=utf-8',
  '.ico': 'image/x-icon',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.js': 'text/javascript; charset=utf-8',
  '.json': 'application/json; charset=utf-8',
  '.map': 'application/json; charset=utf-8',
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

function fileExists(filePath) {
  try { return fs.existsSync(filePath); } catch { return false; }
}
function readText(filePath, fallback = '') {
  try { return fs.readFileSync(filePath, 'utf8'); } catch { return fallback; }
}
function cleanText(value, maxLen = 200) {
  return String(value == null ? '' : value).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}
function normalizeVersionText(value) {
  return cleanText(value, 120).replace(/^[vV]/, '');
}
function parseVersionText(value) {
  const normalized = normalizeVersionText(value);
  const match = normalized.match(/^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/);
  if (!match) return null;
  return {
    raw: normalized,
    major: Number(match[1] || 0),
    minor: Number(match[2] || 0),
    patch: Number(match[3] || 0),
    prerelease: String(match[4] || '').split('.').filter(Boolean),
  };
}
function comparePrereleaseIdentifiers(left, right) {
  var leftText = String(left || '');
  var rightText = String(right || '');
  var leftNum = /^\d+$/.test(leftText);
  var rightNum = /^\d+$/.test(rightText);
  if (leftNum && rightNum) {
    var leftValue = Number(leftText);
    var rightValue = Number(rightText);
    if (leftValue > rightValue) return 1;
    if (leftValue < rightValue) return -1;
    return 0;
  }
  if (leftNum && !rightNum) return -1;
  if (!leftNum && rightNum) return 1;
  if (leftText > rightText) return 1;
  if (leftText < rightText) return -1;
  return 0;
}
function compareVersionText(left, right) {
  var leftParsed = parseVersionText(left);
  var rightParsed = parseVersionText(right);
  if (leftParsed && rightParsed) {
    if (leftParsed.major !== rightParsed.major) return leftParsed.major > rightParsed.major ? 1 : -1;
    if (leftParsed.minor !== rightParsed.minor) return leftParsed.minor > rightParsed.minor ? 1 : -1;
    if (leftParsed.patch !== rightParsed.patch) return leftParsed.patch > rightParsed.patch ? 1 : -1;
    if (!leftParsed.prerelease.length && !rightParsed.prerelease.length) return 0;
    if (!leftParsed.prerelease.length) return 1;
    if (!rightParsed.prerelease.length) return -1;
    var len = Math.max(leftParsed.prerelease.length, rightParsed.prerelease.length);
    for (var i = 0; i < len; i += 1) {
      var leftPart = leftParsed.prerelease[i];
      var rightPart = rightParsed.prerelease[i];
      if (leftPart == null) return -1;
      if (rightPart == null) return 1;
      var cmp = comparePrereleaseIdentifiers(leftPart, rightPart);
      if (cmp !== 0) return cmp;
    }
    return 0;
  }
  var leftNormalized = normalizeVersionText(left);
  var rightNormalized = normalizeVersionText(right);
  if (!leftNormalized && !rightNormalized) return 0;
  if (!leftNormalized) return -1;
  if (!rightNormalized) return 1;
  if (leftNormalized > rightNormalized) return 1;
  if (leftNormalized < rightNormalized) return -1;
  return 0;
}
function readJsonFile(filePath) {
  try {
    return JSON.parse(readText(filePath, '{}') || '{}');
  } catch {
    return null;
  }
}
function versionSourcePriority(source) {
  var key = cleanText(source, 80);
  if (key === 'git_latest_tag') return 40;
  if (key === 'install_release_meta') return 30;
  if (key === 'install_release_tag') return 28;
  if (key === 'runtime_version_contract') return 20;
  if (key === 'package_json') return 10;
  return 0;
}
function buildVersionCandidate(version, tag, source) {
  var normalizedVersion = normalizeVersionText(version);
  if (!normalizedVersion) return null;
  var normalizedTag = cleanText(tag || ('v' + normalizedVersion), 120) || ('v' + normalizedVersion);
  return {
    version: normalizedVersion,
    tag: normalizedTag,
    source: cleanText(source || 'unknown', 80) || 'unknown',
  };
}
function pickHigherVersionCandidate(best, candidate) {
  if (!candidate) return best || null;
  if (!best) return candidate;
  var cmp = compareVersionText(candidate.version, best.version);
  if (cmp > 0) return candidate;
  if (cmp < 0) return best;
  return versionSourcePriority(candidate.source) >= versionSourcePriority(best.source) ? candidate : best;
}
function readGitLatestTagCandidate(workspaceRoot) {
  try {
    var result = childProcess.spawnSync('git', ['tag', '--list', '--sort=-v:refname', 'v*'], {
      cwd: workspaceRoot,
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'ignore'],
    });
    if (!result || result.status !== 0) return null;
    var tag = String(result.stdout || '').split(/\r?\n/).map(function(row) {
      return cleanText(row, 120);
    }).find(Boolean);
    return buildVersionCandidate(tag, tag, 'git_latest_tag');
  } catch {
    return null;
  }
}
function readInstalledReleaseCandidate(workspaceRoot) {
  var metaPath = path.resolve(workspaceRoot, 'local/state/ops/install_release_meta.json');
  var meta = readJsonFile(metaPath);
  if (meta && typeof meta === 'object') {
    var metaValue = cleanText(
      (meta && (meta.release_version_normalized || meta.release_tag)) || '',
      120
    );
    var metaTag = cleanText(meta && meta.release_tag, 120);
    var metaCandidate = buildVersionCandidate(metaValue, metaTag || ('v' + normalizeVersionText(metaValue)), 'install_release_meta');
    if (metaCandidate) return metaCandidate;
  }
  var tagPath = path.resolve(workspaceRoot, 'local/state/ops/install_release_tag.txt');
  var rawTag = cleanText(readText(tagPath, '').split(/\r?\n/)[0] || '', 120);
  return buildVersionCandidate(rawTag, rawTag, 'install_release_tag');
}
function findWorkspaceRoot(startDir) {
  let cursor = path.resolve(startDir || '.');
  for (let hop = 0; hop < 12; hop += 1) {
    const packageJsonPath = path.resolve(cursor, 'package.json');
    if (fileExists(packageJsonPath)) return cursor;
    const next = path.dirname(cursor);
    if (!next || next === cursor) break;
    cursor = next;
  }
  return path.resolve(startDir || '.');
}
function readBuildVersionInfo(staticDir) {
  const workspaceRoot = findWorkspaceRoot(staticDir);
  const runtimeVersionPath = path.resolve(
    workspaceRoot,
    'client',
    'runtime',
    'config',
    'runtime_version.json'
  );
  const packagePath = path.resolve(workspaceRoot, 'package.json');
  let best = null;
  const runtimeVersion = readJsonFile(runtimeVersionPath);
  if (runtimeVersion && typeof runtimeVersion === 'object') {
    best = pickHigherVersionCandidate(
      best,
      buildVersionCandidate(
        runtimeVersion && runtimeVersion.version,
        runtimeVersion && runtimeVersion.tag,
        cleanText(runtimeVersion && runtimeVersion.source, 80) || 'runtime_version_contract'
      )
    );
  }
  const pkg = readJsonFile(packagePath);
  if (pkg && typeof pkg === 'object') {
    best = pickHigherVersionCandidate(
      best,
      buildVersionCandidate(pkg && pkg.version, '', 'package_json')
    );
  }
  best = pickHigherVersionCandidate(best, readInstalledReleaseCandidate(workspaceRoot));
  best = pickHigherVersionCandidate(best, readGitLatestTagCandidate(workspaceRoot));
  if (!best) {
    return {
      version: '0.0.0',
      tag: 'v0.0.0',
      source: 'fallback_default',
    };
  }
  return best;
}
function contentTypeForFile(filePath) {
  return MIME[path.extname(filePath).toLowerCase()] || 'application/octet-stream';
}
function listSegmentPartFiles(basePath) {
  const ext = path.extname(basePath).toLowerCase();
  const partDirs = [`${basePath}.parts`];
  if (ext === '.js') partDirs.push(basePath.replace(/\.js$/i, '.ts') + '.parts');
  if (ext === '.ts') partDirs.push(basePath.replace(/\.ts$/i, '.js') + '.parts');
  for (const partsDir of partDirs) {
    try {
      if (!fs.statSync(partsDir).isDirectory()) continue;
      const rows = fs.readdirSync(partsDir, { withFileTypes: true })
        .filter((entry) => entry.isFile() && path.extname(entry.name).toLowerCase() === ext)
        .map((entry) => path.resolve(partsDir, entry.name))
        .sort((a, b) => a.localeCompare(b, 'en'));
      if (rows.length) return rows;
    } catch {}
  }
  return [];
}
function readSegmentedText(basePath, fallback = '') {
  const partFiles = listSegmentPartFiles(basePath);
  if (partFiles.length) {
    const joined = partFiles.map((filePath) => readText(filePath, '')).filter(Boolean).join('\n');
    if (joined.trim()) return joined;
  }
  return readText(basePath, fallback);
}
function rebrandDashboardText(text) {
  return String(text || '')
    .replace(/\bReference Runtime\b/g, 'Infring')
    .replace(/\bREFERENCE_RUNTIME\b/g, 'INFRING')
    .replace(/\breference_runtime\b/g, 'infring')
    .replace(/\bControl Runtime\b/g, 'Infring')
    .replace(/\bCONTROL_RUNTIME\b/g, 'INFRING')
    .replace(/\bcontrol_runtime\b/g, 'infring');
}
function injectBeforeHeadClose(head, snippet) {
  if (!snippet) return head;
  if (head.includes('</head>')) return head.replace('</head>', `${snippet}\n</head>`);
  return `${head}\n${snippet}`;
}
function readForkScript(staticDir, basePathNoExt) {
  const jsPath = path.resolve(staticDir, `${basePathNoExt}.js`);
  if (fileExists(jsPath) || listSegmentPartFiles(jsPath).length > 0) return readSegmentedText(jsPath, '');
  const tsPath = path.resolve(staticDir, `${basePathNoExt}.ts`);
  return fileExists(tsPath) || listSegmentPartFiles(tsPath).length > 0 ? readSegmentedText(tsPath, '') : '';
}
function agentMutationSyncPatchScript() {
  return [
    '(function(){',
    '  if (window.__infringAgentMutationSyncPatchInstalled) return;',
    '  window.__infringAgentMutationSyncPatchInstalled = true;',
    '  function parseUrl(rawPath) {',
    '    try { return new URL(String(rawPath || \'\'), window.location.origin); } catch(_) { return null; }',
    '  }',
    '  function readStore() {',
    '    try {',
    '      if (window.Alpine && typeof window.Alpine.store === \'function\') return window.Alpine.store(\'app\');',
    '    } catch(_) {}',
    '    return null;',
    '  }',
    '  function triggerForcedRefreshBurst() {',
    '    var delays = [0, 260, 920];',
    '    delays.forEach(function(delay) {',
    '      window.setTimeout(function() {',
    '        var store = readStore();',
    '        if (!store || typeof store.refreshAgents !== \'function\') return;',
    '        Promise.resolve(store.refreshAgents({ force: true })).catch(function() {});',
    '      }, delay);',
    '    });',
    '  }',
    '  function isCreatePath(pathname) {',
    '    if (pathname === \'/api/agents\') return true;',
    '    return /^\\/api\\/agents\\/[^\\/]+\\/(clone|revive)$/.test(pathname);',
    '  }',
    '  function isArchivePath(pathname) {',
    '    return /^\\/api\\/agents\\/[^\\/]+$/.test(pathname);',
    '  }',
    '  function installApiPatch() {',
    '    var api = window.InfringAPI;',
    '    if (!api) return false;',
    '    if (api.__infringAgentMutationSyncPatched) return true;',
    '    api.__infringAgentMutationSyncPatched = true;',
    '    var basePost = typeof api.post === \'function\' ? api.post.bind(api) : null;',
    '    var baseDel = typeof api.del === \'function\' ? api.del.bind(api) : null;',
    '    if (basePost) {',
    '      api.post = function(path, body) {',
    '        var parsed = parseUrl(path);',
    '        return Promise.resolve(basePost(path, body)).then(function(result) {',
    '          var pathname = parsed ? parsed.pathname : String(path || \'\');',
    '          if (isCreatePath(pathname)) triggerForcedRefreshBurst();',
    '          return result;',
    '        });',
    '      };',
    '    }',
    '    if (baseDel) {',
    '      api.del = function(path) {',
    '        var parsed = parseUrl(path);',
    '        return Promise.resolve(baseDel(path)).then(function(result) {',
    '          var pathname = parsed ? parsed.pathname : String(path || \'\');',
    '          if (isArchivePath(pathname)) triggerForcedRefreshBurst();',
    '          return result;',
    '        });',
    '      };',
    '      api.delete = api.del;',
    '    }',
    '    return true;',
    '  }',
    '  if (installApiPatch()) return;',
    '  var attempts = 0;',
    '  var timer = window.setInterval(function() {',
    '    attempts += 1;',
    '    if (installApiPatch() || attempts >= 80) window.clearInterval(timer);',
    '  }, 100);',
    '})();',
  ].join('\n');
}

function hasPrimaryDashboardUi(staticDir) {
  const headPath = path.resolve(staticDir, 'index_head.html');
  const bodyPath = path.resolve(staticDir, 'index_body.html');
  return (fileExists(headPath) || listSegmentPartFiles(headPath).length > 0) && (fileExists(bodyPath) || listSegmentPartFiles(bodyPath).length > 0);
}
function buildPrimaryDashboardHtml(staticDir) {
  const buildVersion = readBuildVersionInfo(staticDir);
  const head = readSegmentedText(path.resolve(staticDir, 'index_head.html'), '');
  const body = readSegmentedText(path.resolve(staticDir, 'index_body.html'), '');
  if (!head || !body) return '';
  const headWithExternalAssets = injectBeforeHeadClose(
    head,
    `<script src="${HIGHLIGHT_JS_CDN_URL}"></script>`
  );
  const css = [
    readSegmentedText(path.resolve(staticDir, 'css/theme.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/layout.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/components.css'), ''),
    readText(path.resolve(staticDir, 'vendor/github-dark.min.css'), ''),
  ].join('\n');
  const scripts = [
    readForkScript(staticDir, 'vendor/marked.min'),
    readForkScript(staticDir, 'vendor/chart.umd.min'),
    readForkScript(staticDir, 'js/svelte/chat_bubble.bundle'),
    readForkScript(staticDir, 'js/api'),
    readForkScript(staticDir, 'js/app'),
    PAGE_SCRIPTS.map((name) => readForkScript(staticDir, `js/pages/${name}`)).filter(Boolean).join('\n'),
  ].filter(Boolean).join('\n');
  const alpine = readForkScript(staticDir, 'vendor/alpine.min');
  const versionBootstrap = [
    'window.__INFRING_BUILD_INFO = ' + JSON.stringify(buildVersion) + ';',
    'window.__INFRING_APP_VERSION = window.__INFRING_BUILD_INFO.version || "0.0.0";',
    'window.__INFRING_APP_TAG = window.__INFRING_BUILD_INFO.tag || ("v" + window.__INFRING_APP_VERSION);',
  ].join('\n');
  return rebrandDashboardText([headWithExternalAssets, '<style>', css, '</style>', body, '<script>', versionBootstrap, '</script>', '<script>', scripts, '</script>', '<script>', alpine, '</script>', '<script>', agentMutationSyncPatchScript(), '</script>', '</body></html>'].join('\n'));
}
function readPrimaryDashboardAsset(staticDir, pathname) {
  const requestPath = pathname === '/' || pathname === '/dashboard' || pathname === '/dashboard/' ? '/index_body.html' : pathname;
  const resolved = path.resolve(staticDir, String(requestPath || '/').replace(/^\/+/, ''));
  const ext = path.extname(resolved).toLowerCase();
  if (!resolved.startsWith(staticDir)) return null;
  if (!fileExists(resolved) && listSegmentPartFiles(resolved).length === 0) return null;
  if (['.js', '.css', '.html', '.json', '.map', '.md', '.txt'].includes(ext)) return { body: rebrandDashboardText(readSegmentedText(resolved, '')), contentType: contentTypeForFile(resolved) };
  return { body: fs.readFileSync(resolved), contentType: contentTypeForFile(resolved) };
}

module.exports = {
  hasPrimaryDashboardUi,
  buildPrimaryDashboardHtml,
  readBuildVersionInfo,
  readPrimaryDashboardAsset,
};
