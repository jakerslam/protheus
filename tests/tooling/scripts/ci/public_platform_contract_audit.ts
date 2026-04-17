#!/usr/bin/env node
/* eslint-disable no-console */
import fs from 'node:fs';
import path from 'node:path';
import { cleanText, hasFlag, parseBool, parseStrictOutArgs, readFlag } from '../../lib/cli.ts';
import { currentRevision } from '../../lib/git.ts';
import { emitStructuredResult } from '../../lib/result.ts';

const ROOT = process.cwd();
const SOURCE_RE = /\.(js|jsx|ts|tsx|py|sh|rs)$/;
const DEFAULT_POLICY_PATH = 'client/runtime/config/public_platform_contract_policy.json';
const DEFAULT_OUT_PATH = 'core/local/artifacts/public_platform_contract_audit_current.json';

function parseArgs(argv: string[]) {
  const common = parseStrictOutArgs(argv, { strict: false });
  const skipPublicRaw = readFlag(argv, 'skip-public');
  return {
    policy: cleanText(readFlag(argv, 'policy') || DEFAULT_POLICY_PATH, 260),
    out: cleanText(readFlag(argv, 'out') || DEFAULT_OUT_PATH, 400),
    docsSurfacePolicy: cleanText(readFlag(argv, 'docs-surface-policy') || '', 260),
    skipPublic: hasFlag(argv, 'skip-public') || parseBool(skipPublicRaw, false),
    strict: common.strict,
  };
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

function buildDocsSurfaceReport(policyPath, revision) {
  const resolved = path.resolve(ROOT, policyPath);
  const policy = JSON.parse(fs.readFileSync(resolved, 'utf8'));
  const violations = [];

  const required = [
    ...(policy.required_operator_docs || []),
    ...(policy.required_public_docs || []),
    ...(policy.required_internal_namespace || []),
  ];

  for (const relPath of required) {
    if (!fs.existsSync(path.resolve(ROOT, relPath))) {
      violations.push({ reason: 'required_doc_missing', path: relPath });
    }
  }

  const readmePath = path.resolve(ROOT, 'README.md');
  const readme = fs.existsSync(readmePath) ? fs.readFileSync(readmePath, 'utf8') : '';
  for (const link of policy.readme_required_links || []) {
    if (!readme.includes(link)) {
      violations.push({ reason: 'readme_required_link_missing', path: link });
    }
  }
  for (const link of policy.readme_forbidden_root_internal_links || []) {
    if (readme.includes(link)) {
      violations.push({ reason: 'readme_forbidden_internal_link_present', path: link });
    }
  }

  for (const [source, target] of Object.entries(policy.internal_aliases || {})) {
    if (!fs.existsSync(path.resolve(ROOT, source))) {
      violations.push({ reason: 'internal_alias_source_missing', path: source, target });
    }
    if (!fs.existsSync(path.resolve(ROOT, target))) {
      violations.push({ reason: 'internal_alias_target_missing', path: target, source });
    }
  }

  const payload = {
    type: 'docs_surface_contract',
    generated_at: new Date().toISOString(),
    revision,
    policy_path: rel(resolved),
    summary: {
      violation_count: violations.length,
      pass: violations.length === 0,
    },
    violations,
  };

  if (policy.paths?.latest_path) {
    const latestPath = path.resolve(ROOT, policy.paths.latest_path);
    fs.mkdirSync(path.dirname(latestPath), { recursive: true });
    fs.writeFileSync(latestPath, `${JSON.stringify(payload, null, 2)}\n`);
  }
  if (policy.paths?.receipts_path) {
    const receiptsPath = path.resolve(ROOT, policy.paths.receipts_path);
    fs.mkdirSync(path.dirname(receiptsPath), { recursive: true });
    fs.appendFileSync(receiptsPath, `${JSON.stringify(payload)}\n`);
  }

  return payload;
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const revision = currentRevision(ROOT);

  if (args.skipPublic && args.docsSurfacePolicy) {
    const payload = buildDocsSurfaceReport(args.docsSurfacePolicy, revision);
    process.exitCode = emitStructuredResult(payload, {
      outPath: args.out,
      strict: args.strict,
      ok: payload.summary.pass,
    });
    return;
  }

  const policyPath = path.resolve(ROOT, args.policy);
  const policy = JSON.parse(fs.readFileSync(policyPath, 'utf8'));

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
    artifact_paths: [args.out].filter(Boolean),
    violations,
  };

  if (args.docsSurfacePolicy) {
    payload.docs_surface_contract = buildDocsSurfaceReport(args.docsSurfacePolicy, revision);
    payload.summary.pass =
      payload.summary.pass && payload.docs_surface_contract.summary.pass;
  }

  process.exitCode = emitStructuredResult(payload, {
    outPath: args.out,
    strict: args.strict,
    ok: payload.summary.pass,
  });
}

main();
