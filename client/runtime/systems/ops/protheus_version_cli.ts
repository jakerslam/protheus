#!/usr/bin/env node
'use strict';

// Layer ownership: client/runtime/systems/ops (authoritative version/update lane)

const fs = require('fs');
const https = require('https');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..', '..', '..');
const PACKAGE_JSON_PATH = path.join(ROOT, 'package.json');
const INSTALL_RELEASE_TAG_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'install_release_tag.txt'
);
const INSTALL_RELEASE_META_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'install_release_meta.json'
);
const RELEASE_CHANNEL_PATH = path.join(
  ROOT,
  'client',
  'runtime',
  'config',
  'protheus_release_channel.json'
);
const STATE_PATH = path.join(
  ROOT,
  'local',
  'state',
  'ops',
  'protheus_version_cli',
  'latest.json'
);
const RELEASES_API_URL = 'https://api.github.com/repos/protheuslabs/InfRing/releases?per_page=12';
const RELEASE_LATEST_API_URL = 'https://api.github.com/repos/protheuslabs/InfRing/releases/latest';
const INSTALL_COMMAND =
  'curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full';
const CACHE_TTL_MS = 6 * 60 * 60 * 1000;

function nowIso() {
  return new Date().toISOString();
}

function cleanText(raw, maxLen = 200) {
  return String(raw || '')
    .trim()
    .replace(/\s+/g, ' ')
    .slice(0, maxLen);
}

function asBool(raw, fallback = false) {
  const normalized = cleanText(raw, 16).toLowerCase();
  if (!normalized) return fallback;
  if (['1', 'true', 'yes', 'on', 'y'].includes(normalized)) return true;
  if (['0', 'false', 'no', 'off', 'n'].includes(normalized)) return false;
  return fallback;
}

function parseArgs(argv = process.argv.slice(2)) {
  const out = {
    command: 'version',
    json: false,
    quiet: false,
    force: false,
    apply: false
  };
  const tokens = Array.isArray(argv)
    ? argv.map((token) => cleanText(token, 160)).filter(Boolean)
    : [];
  if (tokens.length > 0 && !tokens[0].startsWith('--')) {
    out.command = cleanText(tokens.shift(), 40).toLowerCase() || 'version';
  }
  for (const token of tokens) {
    if (token === '--json' || token === '--json=1') {
      out.json = true;
      continue;
    }
    if (token === '--quiet' || token === '--quiet=1') {
      out.quiet = true;
      continue;
    }
    if (token === '--force' || token === '--force=1') {
      out.force = true;
      continue;
    }
    if (token === '--apply' || token === '--apply=1') {
      out.apply = true;
      continue;
    }
    if (token === '--help' || token === '-h') {
      out.command = 'help';
      continue;
    }
  }
  return out;
}

function readJsonFile(filePath, fallback = null) {
  try {
    const raw = fs.readFileSync(filePath, 'utf8');
    const parsed = JSON.parse(raw);
    if (parsed && typeof parsed === 'object') {
      return parsed;
    }
  } catch (_) {}
  return fallback;
}

function writeJsonFile(filePath, payload) {
  try {
    fs.mkdirSync(path.dirname(filePath), { recursive: true });
    fs.writeFileSync(filePath, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  } catch (_) {}
}

function normalizeVersion(raw) {
  return cleanText(raw, 120).replace(/^v/i, '');
}

function readInstalledReleaseMetadata() {
  const meta = readJsonFile(INSTALL_RELEASE_META_PATH, {});
  const releaseVersion = normalizeVersion(
    meta && meta.release_version_normalized ? meta.release_version_normalized : ''
  );
  const releaseTag = normalizeVersion(meta && meta.release_tag ? meta.release_tag : '');
  if (releaseVersion || releaseTag) {
    return {
      releaseVersion: releaseVersion || releaseTag,
      releaseTag: releaseTag || releaseVersion,
      source: 'install_release_meta'
    };
  }
  try {
    const raw = fs.readFileSync(INSTALL_RELEASE_TAG_PATH, 'utf8');
    const tag = normalizeVersion(raw);
    if (tag) {
      return {
        releaseVersion: tag,
        releaseTag: tag,
        source: 'install_release_tag'
      };
    }
  } catch (_) {}
  return null;
}

function readCurrentVersion() {
  const installed = readInstalledReleaseMetadata();
  if (installed && installed.releaseVersion) {
    return installed.releaseVersion;
  }
  const pkg = readJsonFile(PACKAGE_JSON_PATH, {});
  const version = normalizeVersion(pkg && pkg.version ? pkg.version : '');
  return version || '0.0.0-unknown';
}

function parseSemver(raw) {
  const normalized = normalizeVersion(raw);
  const match = /^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.-]+))?$/.exec(normalized);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    prerelease: cleanText(match[4] || '', 80).toLowerCase()
  };
}

function compareSemver(a, b) {
  const pa = parseSemver(a);
  const pb = parseSemver(b);
  if (!pa || !pb) {
    return normalizeVersion(a).localeCompare(normalizeVersion(b));
  }
  if (pa.major !== pb.major) return pa.major > pb.major ? 1 : -1;
  if (pa.minor !== pb.minor) return pa.minor > pb.minor ? 1 : -1;
  if (pa.patch !== pb.patch) return pa.patch > pb.patch ? 1 : -1;
  if (!pa.prerelease && pb.prerelease) return 1;
  if (pa.prerelease && !pb.prerelease) return -1;
  if (!pa.prerelease && !pb.prerelease) return 0;
  return pa.prerelease.localeCompare(pb.prerelease);
}

function firstMeaningfulLine(raw, fallback = '') {
  const lines = String(raw || '')
    .split('\n')
    .map((line) => cleanText(line, 240))
    .filter(Boolean);
  for (const line of lines) {
    if (line.startsWith('#')) continue;
    if (line === '-' || line === '*') continue;
    return line;
  }
  return cleanText(fallback, 240);
}

function readReleaseChannelMetadata() {
  const cfg = readJsonFile(RELEASE_CHANNEL_PATH, {});
  return {
    latestVersion: normalizeVersion(cfg && cfg.latest_version ? cfg.latest_version : ''),
    changelogLine: cleanText(cfg && cfg.changelog_line ? cfg.changelog_line : '', 240),
    releasedAt: cleanText(cfg && cfg.released_at ? cfg.released_at : '', 40)
  };
}

function parseReleasePayload(payload) {
  const latestVersion = normalizeVersion(payload && payload.tag_name ? payload.tag_name : payload.name);
  if (!latestVersion) {
    return null;
  }
  const parsedVersion = parseSemver(latestVersion);
  return {
    latestVersion,
    changelogLine: firstMeaningfulLine(payload && payload.body ? payload.body : '', ''),
    releasedAt: cleanText(
      (payload && (payload.published_at || payload.created_at)) || '',
      40
    ),
    prerelease:
      asBool(payload && payload.prerelease, false) || Boolean(parsedVersion && parsedVersion.prerelease),
    draft: asBool(payload && payload.draft, false)
  };
}

function selectReleaseCandidate(releases, opts = {}) {
  const preferPrerelease = asBool(opts.preferPrerelease, false);
  const candidates = Array.isArray(releases)
    ? releases.map(parseReleasePayload).filter(Boolean).filter((row) => !row.draft)
    : [];
  if (!candidates.length) {
    return null;
  }
  const eligible = candidates.filter((row) => (preferPrerelease ? true : !row.prerelease));
  const pool = eligible.length > 0 ? eligible : candidates;
  let best = null;
  for (const row of pool) {
    if (!best) {
      best = row;
      continue;
    }
    const semverCmp = compareSemver(row.latestVersion, best.latestVersion);
    if (semverCmp > 0) {
      best = row;
      continue;
    }
    if (semverCmp === 0 && row.releasedAt > best.releasedAt) {
      best = row;
    }
  }
  return best;
}

function fetchLatestRelease(timeoutMs = 1800, opts = {}) {
  const preferPrerelease = asBool(opts.preferPrerelease, false);
  return new Promise((resolve) => {
    let settled = false;
    const done = (payload) => {
      if (settled) return;
      settled = true;
      resolve(payload);
    };
    const handleResponse = (source) => (res) => {
      const status = Number(res && res.statusCode ? res.statusCode : 0);
      let body = '';
      res.setEncoding('utf8');
      res.on('data', (chunk) => {
        body += String(chunk || '');
        if (body.length > 512000) {
          body = body.slice(0, 512000);
        }
      });
      res.on('end', () => {
        if (status < 200 || status >= 300) {
          done({
            ok: false,
            error: `github_release_status_${status || 0}`
          });
          return;
        }
        try {
          const parsed = JSON.parse(body || '[]');
          const release = Array.isArray(parsed)
            ? selectReleaseCandidate(parsed, { preferPrerelease })
            : parseReleasePayload(parsed);
          if (!release || !release.latestVersion) {
            done({ ok: false, error: 'github_release_tag_missing' });
            return;
          }
          done({
            ok: true,
            latestVersion: release.latestVersion,
            changelogLine: cleanText(release.changelogLine || '', 240),
            releasedAt: cleanText(release.releasedAt || '', 40),
            prerelease: asBool(release.prerelease, false),
            source
          });
        } catch (_) {
          done({ ok: false, error: 'github_release_parse_failed' });
        }
      });
    };

    const req = https.get(
      RELEASES_API_URL,
      {
        headers: {
          Accept: 'application/vnd.github+json',
          'User-Agent': 'infring-version-cli'
        }
      },
      handleResponse('github_releases_api')
    );
    req.setTimeout(timeoutMs, () => {
      req.destroy(new Error('request_timeout'));
    });
    req.on('error', (err) => {
      const latestReq = https.get(
        RELEASE_LATEST_API_URL,
        {
          headers: {
            Accept: 'application/vnd.github+json',
            'User-Agent': 'infring-version-cli'
          }
        },
        handleResponse('github_latest_api')
      );
      latestReq.setTimeout(timeoutMs, () => {
        latestReq.destroy(new Error('request_timeout'));
      });
      latestReq.on('error', (fallbackErr) => {
        done({
          ok: false,
          error: cleanText(
            `github_release_fetch_failed:${
              fallbackErr && fallbackErr.message ? fallbackErr.message : err && err.message ? err.message : err
            }`,
            220
          )
        });
      });
    });
  });
}

function readCache() {
  const cache = readJsonFile(STATE_PATH, null);
  if (!cache || typeof cache !== 'object') return null;
  const checkedAtMs = Number(cache.checked_at_ms || 0);
  if (!Number.isFinite(checkedAtMs) || checkedAtMs <= 0) return null;
  if (!cache.result || typeof cache.result !== 'object') return null;
  return {
    checkedAtMs,
    result: cache.result
  };
}

function writeCache(result) {
  writeJsonFile(STATE_PATH, {
    type: 'protheus_version_cli_cache',
    schema_version: 1,
    checked_at: nowIso(),
    checked_at_ms: Date.now(),
    result
  });
}

async function resolveReleaseCheck(opts = {}) {
  const force = asBool(opts.force, false);
  const timeoutMs = Number(opts.timeoutMs || 1800);
  const currentVersion = readCurrentVersion();
  const currentParsed = parseSemver(currentVersion);
  const preferPrerelease = Boolean(currentParsed && currentParsed.prerelease);
  const cached = readCache();
  if (!force && cached && Date.now() - cached.checkedAtMs < CACHE_TTL_MS) {
    const cacheVersion = normalizeVersion(cached.result.current_version || '');
    if (cacheVersion === normalizeVersion(currentVersion)) {
      return {
        ...cached.result,
        source: cleanText(cached.result.source || 'cache', 80),
        cache_hit: true
      };
    }
  }

  const localChannel = readReleaseChannelMetadata();
  const remote = await fetchLatestRelease(timeoutMs, { preferPrerelease });

  let latestVersion = currentVersion;
  let changelogLine = '';
  let releasedAt = '';
  let source = 'local_fallback';
  let checkWarning = '';

  if (remote.ok) {
    latestVersion = normalizeVersion(remote.latestVersion) || currentVersion;
    changelogLine = cleanText(remote.changelogLine || '', 240);
    releasedAt = cleanText(remote.releasedAt || '', 40);
    source = cleanText(remote.source || 'github_releases_api', 80);
  } else if (localChannel.latestVersion) {
    latestVersion = localChannel.latestVersion;
    changelogLine = localChannel.changelogLine;
    releasedAt = localChannel.releasedAt;
    source = 'release_channel_config';
    checkWarning = cleanText(remote.error || 'release_check_unavailable', 220);
  } else if (remote.error) {
    checkWarning = cleanText(remote.error, 220);
  }

  const result = {
    ok: true,
    type: 'protheus_version_check',
    current_version: currentVersion,
    latest_version: latestVersion,
    update_available: compareSemver(latestVersion, currentVersion) > 0,
    changelog_line: changelogLine,
    released_at: releasedAt,
    source,
    cache_hit: false,
    checked_at: nowIso()
  };
  if (checkWarning) {
    result.check_warning = checkWarning;
  }
  writeCache(result);
  return result;
}

function printHelp() {
  process.stdout.write('Usage: infring version|update|check-quiet [flags]\n');
  process.stdout.write('\n');
  process.stdout.write('Commands:\n');
  process.stdout.write('  version      Show local version and update signal.\n');
  process.stdout.write('  update       Check latest release and print upgrade command.\n');
  process.stdout.write('  check-quiet  Emit one-line notice only when update is available.\n');
  process.stdout.write('\n');
  process.stdout.write('Flags:\n');
  process.stdout.write('  --json        Print JSON payload.\n');
  process.stdout.write('  --quiet       Suppress advisory notes.\n');
  process.stdout.write('  --force       Bypass cache for remote release check.\n');
  process.stdout.write('  --apply       Run installer command after check (`update` only).\n');
}

function emitJson(payload) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

async function runVersion(opts) {
  const check = await resolveReleaseCheck({ force: opts.force });
  const payload = { ...check, command: 'version' };
  if (opts.json) {
    emitJson(payload);
    return 0;
  }
  process.stdout.write(`infring ${check.current_version}\n`);
  if (check.update_available) {
    process.stdout.write(
      `[infring update] available: ${check.latest_version} (current ${check.current_version})\n`
    );
    if (check.changelog_line) {
      process.stdout.write(`[infring update] ${check.changelog_line}\n`);
    }
  }
  if (check.check_warning && !opts.quiet) {
    process.stdout.write(`[infring update] note: ${check.check_warning}\n`);
  }
  return 0;
}

async function runUpdate(opts) {
  const check = await resolveReleaseCheck({ force: true, timeoutMs: 2200 });
  let applyStatus = 0;
  if (opts.apply) {
    process.stdout.write(`[infring update] applying via: ${INSTALL_COMMAND}\n`);
    const proc = spawnSync('sh', ['-c', INSTALL_COMMAND], {
      cwd: ROOT,
      stdio: 'inherit'
    });
    applyStatus = Number.isFinite(proc.status) ? proc.status : 1;
  }

  const payload = {
    ...check,
    command: 'update',
    install_command: INSTALL_COMMAND,
    apply_requested: opts.apply,
    apply_exit_code: applyStatus
  };

  if (opts.json) {
    emitJson(payload);
    return applyStatus;
  }

  if (check.update_available) {
    process.stdout.write(
      `[infring update] update available: ${check.latest_version} (current ${check.current_version})\n`
    );
    if (check.changelog_line) {
      process.stdout.write(`[infring update] ${check.changelog_line}\n`);
    }
    if (!opts.apply) {
      process.stdout.write(`[infring update] install: ${INSTALL_COMMAND}\n`);
    }
  } else {
    process.stdout.write(`[infring update] already up to date (${check.current_version})\n`);
  }

  if (check.check_warning && !opts.quiet) {
    process.stdout.write(`[infring update] note: ${check.check_warning}\n`);
  }
  return applyStatus;
}

async function runCheckQuiet(opts) {
  const check = await resolveReleaseCheck({ force: opts.force, timeoutMs: 1200 });
  const payload = { ...check, command: 'check-quiet' };
  if (opts.json) {
    emitJson(payload);
    return 0;
  }
  if (check.update_available) {
    process.stdout.write(
      `[infring update] Update available: ${check.latest_version} (current ${check.current_version}). Run: infring update\n`
    );
  }
  return 0;
}

async function main(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  const command = cleanText(opts.command || 'version', 40).toLowerCase();

  if (command === 'help' || command === '--help' || command === '-h') {
    printHelp();
    return 0;
  }
  if (command === 'version' || command === '--version') {
    return runVersion(opts);
  }
  if (command === 'update') {
    return runUpdate(opts);
  }
  if (command === 'check-quiet') {
    return runCheckQuiet(opts);
  }

  const payload = {
    ok: false,
    type: 'protheus_version_cli',
    error: 'unknown_command',
    command: cleanText(command, 40)
  };
  if (opts.json) {
    emitJson(payload);
  } else {
    process.stderr.write(
      `[infring version] unknown subcommand: ${payload.command}. Try: infring version --help\n`
    );
  }
  return 2;
}

if (require.main === module) {
  main(process.argv.slice(2))
    .then((status) => {
      process.exit(Number.isFinite(status) ? status : 1);
    })
    .catch((err) => {
      const msg = cleanText(err && err.message ? err.message : String(err), 220);
      process.stderr.write(`${JSON.stringify({ ok: false, type: 'protheus_version_cli', error: msg })}\n`);
      process.exit(1);
    });
}

module.exports = { main };
