#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');

const ROOT = path.resolve(__dirname, '..', '..');
const PROFILE_PATH = process.env.PROTHEUS_PRIME_PROFILE_PATH
  ? path.resolve(process.env.PROTHEUS_PRIME_PROFILE_PATH)
  : path.join(ROOT, 'config', 'protheus_prime_profile.json');
const RECEIPT_PATH = process.env.PROTHEUS_PRIME_RECEIPT_PATH
  ? path.resolve(process.env.PROTHEUS_PRIME_RECEIPT_PATH)
  : path.join(ROOT, 'state', 'ops', 'protheus_prime_seed', 'latest.json');
const RECEIPT_HISTORY = process.env.PROTHEUS_PRIME_RECEIPT_HISTORY
  ? path.resolve(process.env.PROTHEUS_PRIME_RECEIPT_HISTORY)
  : path.join(ROOT, 'state', 'ops', 'protheus_prime_seed', 'history.jsonl');

type AnyObj = Record<string, any>;

function nowIso() {
  return new Date().toISOString();
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/protheus_prime_seed.js manifest');
  console.log('  node systems/ops/protheus_prime_seed.js bootstrap [--profile=<path>]');
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (const token of argv) {
    if (!token.startsWith('--')) {
      out._.push(token);
      continue;
    }
    const idx = token.indexOf('=');
    if (idx < 0) out[token.slice(2)] = true;
    else out[token.slice(2, idx)] = token.slice(idx + 1);
  }
  return out;
}

function readJson(filePath: string, fallback: any) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function writeJsonAtomic(filePath: string, payload: any) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: any) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function cleanText(v: unknown, maxLen = 320) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeProfile(raw: AnyObj) {
  const src = raw && typeof raw === 'object' ? raw : {};
  return {
    profile_id: cleanText(src.profile_id || 'protheus-prime', 80) || 'protheus-prime',
    version: cleanText(src.version || '1.0', 32) || '1.0',
    mandatory_paths: Array.isArray(src.mandatory_paths)
      ? src.mandatory_paths.map((row: unknown) => cleanText(row, 260)).filter(Boolean)
      : [],
    probes: {
      seed_boot_probe: src.probes && src.probes.seed_boot_probe !== false,
      startup_attestation_verify: src.probes && src.probes.startup_attestation_verify !== false
    }
  };
}

function loadProfile(profilePath = PROFILE_PATH) {
  return normalizeProfile(readJson(profilePath, {}));
}

function runNode(args: string[]) {
  return spawnSync('node', args, {
    cwd: ROOT,
    encoding: 'utf8',
    env: process.env
  });
}

function parseStdoutJson(proc: AnyObj) {
  const text = String(proc && proc.stdout || '').trim();
  if (!text) return null;
  const lines = text.split('\n').map((row) => row.trim()).filter(Boolean);
  for (let i = lines.length - 1; i >= 0; i -= 1) {
    try {
      return JSON.parse(lines[i]);
    } catch {
      // continue
    }
  }
  return null;
}

function evaluateBootstrap(profile: AnyObj) {
  const missing: string[] = [];
  const present: string[] = [];
  for (const relPath of profile.mandatory_paths) {
    const abs = path.isAbsolute(relPath) ? relPath : path.join(ROOT, relPath);
    if (fs.existsSync(abs)) present.push(relPath);
    else missing.push(relPath);
  }

  const probes: AnyObj = {};
  if (profile.probes.seed_boot_probe) {
    const proc = runNode(['systems/ops/seed_boot_probe.js', 'run']);
    probes.seed_boot_probe = {
      status: proc.status,
      ok: proc.status === 0,
      payload: parseStdoutJson(proc)
    };
  }
  if (profile.probes.startup_attestation_verify) {
    const proc = runNode(['systems/security/startup_attestation.js', 'verify']);
    probes.startup_attestation_verify = {
      status: proc.status,
      ok: proc.status === 0,
      payload: parseStdoutJson(proc)
    };
  }

  const drift = probes.seed_boot_probe && probes.seed_boot_probe.payload
    ? Number(probes.seed_boot_probe.payload.boot_ms || 0) > 2000 ? 0.35 : 0.12
    : 0.2;
  const yieldScore = present.length > 0
    ? Number((present.length / Math.max(1, present.length + missing.length)).toFixed(6))
    : 0;
  const safety = probes.startup_attestation_verify && probes.startup_attestation_verify.ok
    ? 1
    : 0.7;
  const integrity = missing.length === 0 ? 1 : Number((1 - (missing.length / Math.max(1, present.length + missing.length))).toFixed(6));

  return {
    ok: missing.length === 0 && Object.values(probes).every((row: any) => row && row.ok === true),
    missing,
    present,
    probes,
    baseline: {
      drift,
      yield: yieldScore,
      safety,
      integrity
    }
  };
}

function persistReceipt(payload: AnyObj) {
  writeJsonAtomic(RECEIPT_PATH, payload);
  appendJsonl(RECEIPT_HISTORY, payload);
}

function cmdManifest(args: AnyObj) {
  const profilePath = args.profile ? path.resolve(String(args.profile)) : PROFILE_PATH;
  const profile = loadProfile(profilePath);
  const out = {
    ok: true,
    type: 'protheus_prime_manifest',
    ts: nowIso(),
    profile_path: profilePath,
    profile
  };
  process.stdout.write(`${JSON.stringify(out)}\n`);
}

function cmdBootstrap(args: AnyObj) {
  const profilePath = args.profile ? path.resolve(String(args.profile)) : PROFILE_PATH;
  const profile = loadProfile(profilePath);
  const evalOut = evaluateBootstrap(profile);
  const payload = {
    ok: evalOut.ok,
    type: 'protheus_prime_bootstrap',
    ts: nowIso(),
    profile_path: profilePath,
    profile_id: profile.profile_id,
    profile_version: profile.version,
    missing_paths: evalOut.missing,
    present_paths: evalOut.present,
    probes: evalOut.probes,
    baseline: evalOut.baseline
  };
  persistReceipt(payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  if (!evalOut.ok) process.exit(1);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || '').trim();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h' || args.help) {
    usage();
    process.exit(0);
  }
  if (cmd === 'manifest') return cmdManifest(args);
  if (cmd === 'bootstrap') return cmdBootstrap(args);
  usage();
  process.exit(2);
}

if (require.main === module) {
  main();
}
