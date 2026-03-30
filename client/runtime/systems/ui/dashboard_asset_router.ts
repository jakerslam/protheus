#!/usr/bin/env tsx

const fs = require('node:fs');
const path = require('node:path');

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
    .replace(/\bOpenFang\b/g, 'Infring').replace(/\bOPENFANG\b/g, 'INFRING').replace(/\bopenfang\b/g, 'infring')
    .replace(/\bOpenClaw\b/g, 'Infring').replace(/\bOPENCLAW\b/g, 'INFRING').replace(/\bopenclaw\b/g, 'infring');
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
  const head = readSegmentedText(path.resolve(staticDir, 'index_head.html'), '');
  const body = readSegmentedText(path.resolve(staticDir, 'index_body.html'), '');
  if (!head || !body) return '';
  const css = [
    readSegmentedText(path.resolve(staticDir, 'css/theme.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/layout.css'), ''),
    readSegmentedText(path.resolve(staticDir, 'css/components.css'), ''),
    readText(path.resolve(staticDir, 'vendor/github-dark.min.css'), ''),
  ].join('\n');
  const scripts = [
    readForkScript(staticDir, 'vendor/marked.min'),
    readForkScript(staticDir, 'vendor/highlight.min'),
    readForkScript(staticDir, 'vendor/chart.umd.min'),
    readForkScript(staticDir, 'js/api'),
    readForkScript(staticDir, 'js/app'),
    PAGE_SCRIPTS.map((name) => readForkScript(staticDir, `js/pages/${name}`)).filter(Boolean).join('\n'),
  ].filter(Boolean).join('\n');
  const alpine = readForkScript(staticDir, 'vendor/alpine.min');
  return rebrandDashboardText([head, '<style>', css, '</style>', body, '<script>', scripts, '</script>', '<script>', alpine, '</script>', '<script>', agentMutationSyncPatchScript(), '</script>', '</body></html>'].join('\n'));
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
  readPrimaryDashboardAsset,
};
