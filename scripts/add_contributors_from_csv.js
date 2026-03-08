#!/usr/bin/env node
'use strict';

// FIXME(rk): Consider adding validation for CSV encoding edge cases (UTF-8 BOM, quoted newlines)
// See docs/ops/CONTRIBUTOR_IMPORT.md for current handling. Tracked under OPS-284.
const fs = require('fs');
const path = require('path');

function parseArgs(argv) {
  const args = { csv: '', role: 'code', outManifest: 'client/docs/community/contributors_manifest.json' };
  for (let i = 2; i < argv.length; i += 1) {
    const token = argv[i];
    if (token.startsWith('--csv=')) args.csv = token.slice('--csv='.length);
    else if (token.startsWith('--role=')) args.role = token.slice('--role='.length);
    else if (token.startsWith('--out-manifest=')) args.outManifest = token.slice('--out-manifest='.length);
    else if (token === '--help' || token === '-h') {
      console.log('Usage: node scripts/add_contributors_from_csv.js --csv=<path> [--role=code] [--out-manifest=client/docs/community/contributors_manifest.json]');
      process.exit(0);
    }
  }
  if (!args.csv) {
    throw new Error('missing --csv=<path>');
  }
  return args;
}

function parseCsv(content) {
  const lines = content.split(/\r?\n/).filter((line) => line.trim().length > 0);
  if (lines.length < 2) {
    throw new Error('csv must include header and at least one row');
  }
  const headers = lines[0].split(',').map((h) => h.trim());
  const required = ['github_username', 'role', 'consent_token'];
  for (const key of required) {
    if (!headers.includes(key)) {
      throw new Error(`csv missing required header: ${key}`);
    }
  }
  const records = [];
  for (let i = 1; i < lines.length; i += 1) {
    const cols = lines[i].split(',').map((v) => v.trim());
    const row = {};
    headers.forEach((h, idx) => {
      row[h] = cols[idx] || '';
    });
    records.push(row);
  }
  return records;
}

function validateUsername(username) {
  // GitHub login shape: 1-39 chars, alnum or '-', no leading/trailing hyphen.
  return /^[A-Za-z0-9](?:[A-Za-z0-9-]{0,37}[A-Za-z0-9])?$/.test(username);
}

function normalizeRole(value, defaultRole) {
  const raw = (value || defaultRole || 'code').trim();
  const parts = raw.split(/[;|]/).map((p) => p.trim()).filter(Boolean);
  return parts.length ? parts : [defaultRole || 'code'];
}

function ensureDir(filePath) {
  const dir = path.dirname(filePath);
  fs.mkdirSync(dir, { recursive: true });
}

function readJsonMaybe(filePath) {
  if (!fs.existsSync(filePath)) return null;
  return JSON.parse(fs.readFileSync(filePath, 'utf8'));
}

function buildAllContributors(existing, entries) {
  const base = existing && typeof existing === 'object' ? existing : {};
  const result = {
    projectName: base.projectName || 'protheus',
    projectOwner: base.projectOwner || 'protheuslabs',
    repoType: base.repoType || 'github',
    repoHost: base.repoHost || 'https://github.com',
    files: Array.isArray(base.files) && base.files.length ? base.files : ['README.md'],
    contributors: entries,
    contributorsPerLine: base.contributorsPerLine || 6,
    skipCi: Boolean(base.skipCi)
  };
  return result;
}

function main() {
  const args = parseArgs(process.argv);
  const csvPath = path.resolve(args.csv);
  const manifestPath = path.resolve(args.outManifest);
  const rcPath = path.resolve('.all-contributorsrc');

  const rows = parseCsv(fs.readFileSync(csvPath, 'utf8'));
  const seen = new Set();
  const contributors = [];

  for (const row of rows) {
    const username = (row.github_username || '').trim();
    if (!validateUsername(username)) {
      throw new Error(`invalid github_username: ${username || '<empty>'}`);
    }
    if (seen.has(username.toLowerCase())) {
      throw new Error(`duplicate github_username: ${username}`);
    }
    seen.add(username.toLowerCase());

    const consentToken = (row.consent_token || '').trim();
    if (!consentToken) {
      throw new Error(`missing consent_token for github_username=${username}`);
    }

    const contributions = normalizeRole(row.role, args.role);
    contributors.push({
      login: username,
      name: (row.name || username).trim(),
      contributions,
      consent_token: consentToken,
      joined_at: (row.joined_at || new Date().toISOString().slice(0, 10)).trim()
    });
  }

  contributors.sort((a, b) => a.login.localeCompare(b.login));

  const existingRc = readJsonMaybe(rcPath);
  const rc = buildAllContributors(existingRc, contributors.map((c) => ({
    login: c.login,
    name: c.name,
    contributions: c.contributions
  })));

  ensureDir(manifestPath);
  fs.writeFileSync(manifestPath, JSON.stringify({
    generated_at: new Date().toISOString(),
    source_csv: path.relative(process.cwd(), csvPath),
    contributor_count: contributors.length,
    contributors
  }, null, 2) + '\n');

  fs.writeFileSync(rcPath, JSON.stringify(rc, null, 2) + '\n');

  console.log(JSON.stringify({
    ok: true,
    csv: path.relative(process.cwd(), csvPath),
    all_contributorsrc: '.all-contributorsrc',
    manifest: path.relative(process.cwd(), manifestPath),
    contributor_count: contributors.length
  }, null, 2));
}

if (require.main === module) {
  try {
    main();
  } catch (error) {
    console.error(JSON.stringify({ ok: false, error: error.message }, null, 2));
    process.exit(1);
  }
}
