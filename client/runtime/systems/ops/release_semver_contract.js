#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

function findRepoRoot(startDir) {
  let dir = path.resolve(startDir || process.cwd());
  while (true) {
    if (fs.existsSync(path.join(dir, '.git')) && fs.existsSync(path.join(dir, 'package.json'))) {
      return dir;
    }
    const parent = path.dirname(dir);
    if (parent === dir) return path.resolve(startDir || process.cwd());
    dir = parent;
  }
}

const CLIENT_ROOT = path.resolve(__dirname, '..', '..');
const ROOT = findRepoRoot(CLIENT_ROOT);
const DEFAULT_PLAN_PATH = path.join(CLIENT_ROOT, 'state', 'release', 'semantic_release_plan.json');
const DEFAULT_CHANGELOG_PATH = path.join(CLIENT_ROOT, 'state', 'release', 'CHANGELOG.auto.md');

function parseArgs(argv) {
  const args = {
    command: 'run',
    strict: false,
    write: true,
    planPath: process.env.RELEASE_SEMVER_PLAN_PATH || DEFAULT_PLAN_PATH,
    changelogPath: process.env.RELEASE_SEMVER_CHANGELOG_PATH || DEFAULT_CHANGELOG_PATH
  };
  const parts = argv.slice(2);
  if (parts.length > 0 && !parts[0].startsWith('--')) args.command = parts[0];
  for (const raw of parts) {
    if (!raw.startsWith('--')) continue;
    const [key, value = '1'] = raw.slice(2).split('=');
    if (key === 'strict') args.strict = value === '1' || value === 'true';
    else if (key === 'write') args.write = value === '1' || value === 'true';
    else if (key === 'plan') args.planPath = path.resolve(ROOT, value);
    else if (key === 'changelog') args.changelogPath = path.resolve(ROOT, value);
  }
  return args;
}

function runGit(args, strict = true) {
  const res = spawnSync('git', args, { cwd: ROOT, encoding: 'utf8' });
  if (strict && res.status !== 0) {
    throw new Error(`git_failed:${args.join(' ')}:${(res.stderr || '').trim()}`);
  }
  return String(res.stdout || '').trim();
}

function latestSemverTag() {
  const raw = runGit(['tag', '--sort=-version:refname', '-l', 'v[0-9]*.[0-9]*.[0-9]*'], false);
  const first = raw.split('\n').map((v) => v.trim()).filter(Boolean)[0];
  return first || 'v0.0.0';
}

function commitRange(tag) {
  if (!tag || tag === 'v0.0.0') return 'HEAD';
  return `${tag}..HEAD`;
}

function collectCommits(range) {
  const out = runGit(['log', '--pretty=format:%H%x1f%s%x1f%b%x1e', range], false);
  if (!out) return [];
  return out
    .split('\x1e')
    .map((row) => row.trim())
    .filter(Boolean)
    .map((row) => {
      const [sha = '', subject = '', body = ''] = row.split('\x1f');
      return { sha, subject: subject.trim(), body: body.trim() };
    });
}

function conventionalType(subject) {
  const match = /^([a-z]+)(\(.+\))?(!)?:\s+/.exec(subject);
  if (!match) return null;
  return { type: match[1], breakingBang: match[3] === '!' };
}

function classifyBump(commits) {
  let bump = null;
  const typed = [];
  for (const c of commits) {
    const parsed = conventionalType(c.subject);
    if (!parsed) continue;
    const breaking = parsed.breakingBang || /BREAKING CHANGE/i.test(c.body);
    typed.push({ ...c, type: parsed.type, breaking });
    if (breaking) {
      bump = 'major';
      continue;
    }
    if (bump !== 'major' && parsed.type === 'feat') bump = 'minor';
    if (!bump && ['fix', 'perf', 'refactor', 'docs', 'build', 'ci', 'test', 'chore', 'style'].includes(parsed.type)) {
      bump = 'patch';
    }
  }
  return { bump, typed };
}

function nextVersion(tag, bump) {
  const clean = (tag || 'v0.0.0').replace(/^v/, '');
  const [ma = '0', mi = '0', pa = '0'] = clean.split('.');
  let major = Number(ma);
  let minor = Number(mi);
  let patch = Number(pa);
  if (bump === 'major') {
    major += 1;
    minor = 0;
    patch = 0;
  } else if (bump === 'minor') {
    minor += 1;
    patch = 0;
  } else if (bump === 'patch') {
    patch += 1;
  }
  return `v${major}.${minor}.${patch}`;
}

function renderChangelog(nextTag, prevTag, typedCommits) {
  const groups = new Map();
  for (const c of typedCommits) {
    const key = c.breaking ? 'breaking' : c.type;
    if (!groups.has(key)) groups.set(key, []);
    groups.get(key).push(c);
  }
  const order = ['breaking', 'feat', 'fix', 'perf', 'refactor', 'docs', 'build', 'ci', 'test', 'chore', 'style'];
  const titles = {
    breaking: 'Breaking Changes',
    feat: 'Features',
    fix: 'Fixes',
    perf: 'Performance',
    refactor: 'Refactors',
    docs: 'Documentation',
    build: 'Build',
    ci: 'CI',
    test: 'Tests',
    chore: 'Chores',
    style: 'Style'
  };
  const lines = [];
  lines.push(`# ${nextTag}`);
  lines.push('');
  lines.push(`Generated from conventional commits in \`${prevTag}..HEAD\`.`);
  lines.push('');
  for (const key of order) {
    const rows = groups.get(key) || [];
    if (!rows.length) continue;
    lines.push(`## ${titles[key] || key}`);
    for (const row of rows) {
      lines.push(`- ${row.subject} (${row.sha.slice(0, 7)})`);
    }
    lines.push('');
  }
  if (typedCommits.length === 0) {
    lines.push('No conventional commits detected.');
    lines.push('');
  }
  return lines.join('\n');
}

function writeFile(p, body) {
  fs.mkdirSync(path.dirname(p), { recursive: true });
  fs.writeFileSync(p, body);
}

function run(args) {
  const previousTag = latestSemverTag();
  const range = commitRange(previousTag);
  const commits = collectCommits(range);
  const { bump, typed } = classifyBump(commits);
  const nextTag = bump ? nextVersion(previousTag, bump) : previousTag;
  const plan = {
    schema_id: 'release_semver_contract_result',
    schema_version: '1.0.0',
    generated_at: new Date().toISOString(),
    previous_tag: previousTag,
    bump: bump || 'none',
    next_tag: nextTag,
    commit_count: commits.length,
    conventional_commit_count: typed.length,
    release_ready: Boolean(bump),
    range
  };
  const changelog = renderChangelog(nextTag, previousTag, typed);
  if (args.write) {
    writeFile(args.planPath, `${JSON.stringify(plan, null, 2)}\n`);
    writeFile(args.changelogPath, `${changelog}\n`);
  }
  process.stdout.write(`${JSON.stringify({ ...plan, plan_path: path.relative(ROOT, args.planPath), changelog_path: path.relative(ROOT, args.changelogPath) }, null, 2)}\n`);
  if (args.strict && !plan.release_ready) process.exit(1);
}

function status(args) {
  if (!fs.existsSync(args.planPath)) {
    process.stdout.write(`${JSON.stringify({ schema_id: 'release_semver_contract_result', ok: false, reason: 'plan_missing' }, null, 2)}\n`);
    process.exit(1);
  }
  process.stdout.write(fs.readFileSync(args.planPath, 'utf8'));
}

function main() {
  const args = parseArgs(process.argv);
  if (args.command === 'run') return run(args);
  if (args.command === 'status') return status(args);
  process.stderr.write('usage: node client/runtime/systems/ops/release_semver_contract.js [run|status] [--strict=1] [--write=1] [--plan=path] [--changelog=path]\n');
  process.exit(1);
}

main();
