#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { execSync } from 'node:child_process';

const ROOT = process.cwd();
const SOURCE_RE = /\.(js|jsx|ts|tsx|py|sh|rs)$/;

function parseArgs(argv) {
  const out = {
    policy: 'client/runtime/config/public_platform_contract_policy.json',
    out: '',
    strict: false,
  };
  for (const arg of argv) {
    if (arg.startsWith('--policy=')) out.policy = arg.slice('--policy='.length);
    else if (arg.startsWith('--out=')) out.out = arg.slice('--out='.length);
    else if (arg.startsWith('--strict=')) {
      const v = String(arg.slice('--strict='.length)).toLowerCase();
      out.strict = ['1', 'true', 'yes', 'on'].includes(v);
    } else if (arg === '--strict') out.strict = true;
  }
  return out;
}

function rel(p) {
  return path.relative(ROOT, p).replace(/\\/g, '/');
}

function walk(dir, out = []) {
  if (!fs.existsSync(dir)) return out;
  for (const ent of fs.readdirSync(dir, { withFileTypes: true })) {
    const abs = path.join(dir, ent.name);
    if (ent.isDirectory()) {
      if (fs.existsSync(path.join(abs, '.git'))) continue;
      walk(abs, out);
      continue;
    }
    if (SOURCE_RE.test(ent.name)) out.push(abs);
  }
  return out;
}

function normalizeLiteral(literal) {
  return String(literal || '').replace(/^\.\//, '').replace(/\\/g, '/');
}

function extractStringLiterals(source) {
  const literals = [];
  const re = /['"`]([^'"`\n]+)['"`]/g;
  let match;
  while ((match = re.exec(source))) {
    literals.push(match[1]);
  }
  return literals;
}

function startsWithAny(value, prefixes) {
  return prefixes.some((prefix) => value.startsWith(prefix));
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));

  let revision = 'unknown';
  try {
    revision = execSync('git rev-parse HEAD', { cwd: ROOT, encoding: 'utf8' }).trim();
  } catch {}

  const scanRoots = Array.isArray(policy.scan_roots) ? policy.scan_roots : [];
  const allowedClientPrefixes = Array.isArray(policy.allowed_client_prefixes)
    ? policy.allowed_client_prefixes
    : [];
  const forbiddenPrefixes = Array.isArray(policy.forbidden_prefixes) ? policy.forbidden_prefixes : [];
  const ignorePrefixes = Array.isArray(policy.ignore_path_prefixes) ? policy.ignore_path_prefixes : [];

  const files = [];
  for (const root of scanRoots) files.push(...walk(path.resolve(ROOT, root)));

  const violations = [];
  for (const abs of files) {
    const file = rel(abs);
    if (startsWithAny(file, ignorePrefixes)) continue;
    const source = fs.readFileSync(abs, 'utf8');
    const literals = extractStringLiterals(source).map(normalizeLiteral);
    const seen = new Set();

    for (const literal of literals) {
      if (!literal || seen.has(literal)) continue;
      seen.add(literal);
      if (!startsWithAny(literal, forbiddenPrefixes)) continue;
      if (startsWithAny(literal, allowedClientPrefixes)) continue;
      violations.push({
        file,
        literal,
        reason: 'app_or_adapter_reaches_private_surface',
      });
    }
  }

  const payload = {
    type: 'public_platform_contract_audit',
    generated_at: new Date().toISOString(),
    revision,
    policy_path: rel(policyPath),
    summary: {
      scanned_files: files.length,
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    violations,
  };

  if (args.out) {
    const outPath = path.resolve(ROOT, args.out);
    fs.mkdirSync(path.dirname(outPath), { recursive: true });
    fs.writeFileSync(outPath, `${JSON.stringify(payload, null, 2)}\n`);
  }

  console.log(JSON.stringify(payload, null, 2));
  if (args.strict && violations.length > 0) process.exit(1);
}

main();
