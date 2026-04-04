#!/usr/bin/env tsx

const fs = require('node:fs');
const path = require('node:path');
const { execFileSync } = require('node:child_process');

const ROOT = path.resolve(__dirname, '../../../../');
const PACKAGE_JSON_PATH = path.resolve(ROOT, 'package.json');
const PACKAGE_LOCK_PATH = path.resolve(ROOT, 'package-lock.json');
const RUNTIME_VERSION_PATH = path.resolve(
  ROOT,
  'client/runtime/config/runtime_version.json'
);

function readJson(filePath, fallback = null) {
  try {
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function writeJson(filePath, value) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
  fs.writeFileSync(filePath, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
}

function cleanText(value, maxLen = 2000) {
  return String(value == null ? '' : value)
    .replace(/\s+/g, ' ')
    .trim()
    .slice(0, maxLen);
}

function parseBool(value, fallback = false) {
  const raw = cleanText(value, 24).toLowerCase();
  if (!raw) return fallback;
  return raw === '1' || raw === 'true' || raw === 'yes' || raw === 'on';
}

function runGit(args, fallback = '') {
  try {
    return String(
      execFileSync('git', args, {
        cwd: ROOT,
        encoding: 'utf8',
        stdio: ['ignore', 'pipe', 'pipe'],
      }) || ''
    ).trim();
  } catch {
    return fallback;
  }
}

function parseSemver(raw) {
  const clean = cleanText(raw, 120).replace(/^v/i, '');
  const match = clean.match(/^(\d+)\.(\d+)\.(\d+)(?:[-+].*)?$/);
  if (!match) return null;
  return {
    major: Number(match[1]),
    minor: Number(match[2]),
    patch: Number(match[3]),
    normalized: `${match[1]}.${match[2]}.${match[3]}`,
  };
}

function latestSemverTag() {
  const output = runGit(['tag', '--list', '--sort=-v:refname', 'v*'], '');
  const tags = output
    .split('\n')
    .map((row) => cleanText(row, 128))
    .filter(Boolean);
  for (const tag of tags) {
    if (parseSemver(tag)) return tag;
  }
  return '';
}

function baseVersionTriplet(previousTag) {
  const fromTag = parseSemver(previousTag || '');
  if (fromTag) return fromTag;
  const fromPackage = parseSemver(
    readJson(PACKAGE_JSON_PATH, {})?.version || '0.0.0'
  );
  if (fromPackage) return fromPackage;
  return { major: 0, minor: 0, patch: 0, normalized: '0.0.0' };
}

function readCommitRows(rangeExpr) {
  const format = '%H%x1f%s%x1f%b%x1e';
  const args = ['log', '--format=' + format];
  if (rangeExpr) args.push(rangeExpr);
  const raw = runGit(args, '');
  const chunks = String(raw || '')
    .split('\x1e')
    .map((row) => row.trim())
    .filter(Boolean);
  return chunks
    .map((row) => {
      const parts = row.split('\x1f');
      return {
        sha: cleanText(parts[0] || '', 80),
        subject: cleanText(parts[1] || '', 400),
        body: cleanText(parts[2] || '', 6000),
      };
    })
    .filter((row) => row.sha && row.subject);
}

function isReleaseChore(subject) {
  const s = cleanText(subject, 220).toLowerCase();
  return /^chore\(release\):\s*v\d+\.\d+\.\d+/.test(s);
}

function conventionalType(subject) {
  const s = cleanText(subject, 260);
  const match = s.match(/^([a-z]+)(?:\([^)]+\))?(!)?:/i);
  if (!match) return { type: '', breakingBang: false };
  return {
    type: String(match[1] || '').toLowerCase(),
    breakingBang: !!match[2],
  };
}

function isBreakingChange(subject, body) {
  const info = conventionalType(subject);
  if (info.breakingBang) return true;
  const b = cleanText(body, 6000);
  return /(^|\n)\s*BREAKING[\s_-]CHANGE\s*:/i.test(b);
}

function classifyBump(commits) {
  let sawMinor = false;
  let sawPatch = false;
  for (const row of commits) {
    const subject = cleanText(row.subject, 260);
    const body = cleanText(row.body, 6000);
    if (isReleaseChore(subject)) continue;
    if (isBreakingChange(subject, body)) return 'major';
    const info = conventionalType(subject);
    if (info.type === 'feat') sawMinor = true;
    else sawPatch = true;
  }
  if (sawMinor) return 'minor';
  if (sawPatch) return 'patch';
  return 'none';
}

function bumpVersion(base, bumpKind) {
  const major = Number(base.major || 0);
  const minor = Number(base.minor || 0);
  const patch = Number(base.patch || 0);
  if (bumpKind === 'major') return `${major + 1}.0.0`;
  if (bumpKind === 'minor') return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

function updatePackageVersion(version) {
  const pkg = readJson(PACKAGE_JSON_PATH, null);
  if (!pkg || typeof pkg !== 'object') return false;
  if (cleanText(pkg.version, 100) === version) return false;
  pkg.version = version;
  writeJson(PACKAGE_JSON_PATH, pkg);
  return true;
}

function updatePackageLockVersion(version) {
  const lock = readJson(PACKAGE_LOCK_PATH, null);
  if (!lock || typeof lock !== 'object') return false;
  let changed = false;
  if (cleanText(lock.version, 100) !== version) {
    lock.version = version;
    changed = true;
  }
  if (lock.packages && lock.packages[''] && typeof lock.packages[''] === 'object') {
    if (cleanText(lock.packages[''].version, 100) !== version) {
      lock.packages[''].version = version;
      changed = true;
    }
  }
  if (changed) writeJson(PACKAGE_LOCK_PATH, lock);
  return changed;
}

function writeRuntimeVersionData(version, bumpKind, previousTag, nextTag, releaseReady) {
  const payload = {
    schema_version: 1,
    version: version,
    tag: nextTag || `v${version}`,
    previous_tag: previousTag || null,
    bump: bumpKind,
    release_ready: !!releaseReady,
    source: 'release_semver_contract',
  };
  const prior = readJson(RUNTIME_VERSION_PATH, null);
  const before = prior ? JSON.stringify(prior) : '';
  const after = JSON.stringify(payload);
  if (before === after) return false;
  writeJson(RUNTIME_VERSION_PATH, payload);
  return true;
}

function parseArgs(argv) {
  const out = {
    command: 'run',
    write: false,
    strict: false,
    pretty: true,
  };
  let commandSet = false;
  for (const tokenRaw of Array.isArray(argv) ? argv : []) {
    const token = cleanText(tokenRaw, 300);
    if (!token) continue;
    if (!commandSet && !token.startsWith('--')) {
      out.command = token.toLowerCase();
      commandSet = true;
      continue;
    }
    if (token.startsWith('--write=')) out.write = parseBool(token.slice(8), false);
    else if (token.startsWith('--strict=')) out.strict = parseBool(token.slice(9), false);
    else if (token.startsWith('--pretty=')) out.pretty = parseBool(token.slice(9), true);
  }
  return out;
}

function buildPlan() {
  const previousTag = latestSemverTag();
  const range = previousTag ? `${previousTag}..HEAD` : 'HEAD';
  const commits = readCommitRows(range).filter((row) => !isReleaseChore(row.subject));
  const bump = classifyBump(commits);
  const releaseReady = bump !== 'none' && commits.length > 0;
  const base = baseVersionTriplet(previousTag);
  const nextVersion = releaseReady ? bumpVersion(base, bump) : base.normalized;
  const nextTag = releaseReady ? `v${nextVersion}` : 'none';
  return {
    ok: true,
    mode: 'conventional_commits',
    release_ready: releaseReady,
    previous_tag: previousTag || 'none',
    next_tag: nextTag,
    current_version: base.normalized,
    next_version: nextVersion,
    bump: bump,
    commits_scanned: commits.length,
    commits: commits.slice(0, 60).map((row) => ({
      sha: row.sha,
      subject: row.subject,
      classification: isBreakingChange(row.subject, row.body)
        ? 'major'
        : conventionalType(row.subject).type === 'feat'
        ? 'minor'
        : 'patch',
    })),
  };
}

function run(argv = process.argv.slice(2)) {
  const opts = parseArgs(argv);
  const plan = buildPlan();
  let wroteVersion = false;
  if (opts.write && plan.release_ready) {
    wroteVersion = updatePackageVersion(plan.next_version) || wroteVersion;
    wroteVersion = updatePackageLockVersion(plan.next_version) || wroteVersion;
    wroteVersion =
      writeRuntimeVersionData(
        plan.next_version,
        plan.bump,
        plan.previous_tag,
        plan.next_tag,
        plan.release_ready
      ) || wroteVersion;
  } else if (opts.write) {
    writeRuntimeVersionData(
      plan.release_ready ? plan.next_version : plan.current_version,
      plan.bump,
      plan.previous_tag,
      plan.next_tag,
      plan.release_ready
    );
  }
  const out = {
    ...plan,
    write_requested: opts.write,
    version_bumped: wroteVersion,
  };
  process.stdout.write(
    `${opts.pretty ? JSON.stringify(out, null, 2) : JSON.stringify(out)}\n`
  );
  if (opts.strict && !out.ok) return 1;
  return 0;
}

if (require.main === module) {
  process.exitCode = run(process.argv.slice(2));
}

module.exports = {
  run,
  buildPlan,
};
