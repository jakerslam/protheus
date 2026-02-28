#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { spawnSync } = require('child_process');
let sovereignBlockchainBridge: AnyObj = null;
try {
  sovereignBlockchainBridge = require('../blockchain/sovereign_blockchain_bridge.js');
} catch {
  sovereignBlockchainBridge = null;
}

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
const PACKAGE_DIR = process.env.PROTHEUS_PRIME_PACKAGE_DIR
  ? path.resolve(process.env.PROTHEUS_PRIME_PACKAGE_DIR)
  : path.join(ROOT, 'state', 'ops', 'protheus_prime_seed', 'packages');
const PROVISION_DIR = process.env.PROTHEUS_PRIME_PROVISION_DIR
  ? path.resolve(process.env.PROTHEUS_PRIME_PROVISION_DIR)
  : path.join(ROOT, 'state', 'ops', 'protheus_prime_seed', 'provisioned', 'latest');

type AnyObj = Record<string, any>;

function nowIso() {
  return new Date().toISOString();
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/ops/protheus_prime_seed.js manifest');
  console.log('  node systems/ops/protheus_prime_seed.js bootstrap [--profile=<path>] [--provision-dir=<path>] [--no-provision]');
  console.log('  node systems/ops/protheus_prime_seed.js package [--profile=<path>] [--strict=1|0] [--out-dir=<path>]');
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

function sha256File(filePath: string) {
  const data = fs.readFileSync(filePath);
  return crypto.createHash('sha256').update(data).digest('hex');
}

function safeRelPath(relPath: string) {
  const clean = String(relPath || '').replace(/\\/g, '/').replace(/^\/+/, '');
  if (!clean || clean.includes('..')) return null;
  return clean;
}

function copyFileToProvision(relPath: string, targetRoot: string) {
  const safeRel = safeRelPath(relPath);
  if (!safeRel) return null;
  const src = path.join(ROOT, safeRel);
  if (!fs.existsSync(src)) return null;
  const dest = path.join(targetRoot, safeRel);
  ensureDir(path.dirname(dest));
  fs.copyFileSync(src, dest);
  return {
    path: safeRel,
    size_bytes: fs.statSync(dest).size,
    sha256: sha256File(dest)
  };
}

function cleanText(v: unknown, maxLen = 320) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v: unknown, maxLen = 160) {
  return cleanText(v, maxLen)
    .toLowerCase()
    .replace(/[^a-z0-9_.:/-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function normalizeProfile(raw: AnyObj) {
  const src = raw && typeof raw === 'object' ? raw : {};
  const mandatoryPaths = Array.isArray(src.mandatory_paths)
    ? src.mandatory_paths.map((row: unknown) => cleanText(row, 260)).filter(Boolean)
    : [];
  const governancePaths = Array.isArray(src.mandatory_governance_paths)
    ? src.mandatory_governance_paths.map((row: unknown) => cleanText(row, 260)).filter(Boolean)
    : mandatoryPaths.filter((row) =>
      /AGENT-CONSTITUTION\.md|systems\/eye\/eye_kernel|systems\/security\/guard|systems\/security\/startup_attestation|systems\/workflow\/workflow_executor/i.test(row)
    );
  return {
    profile_id: cleanText(src.profile_id || 'protheus-prime', 80) || 'protheus-prime',
    version: cleanText(src.version || '1.0', 32) || '1.0',
    mandatory_paths: mandatoryPaths,
    mandatory_governance_paths: governancePaths,
    provision_on_bootstrap: src.provision_on_bootstrap === false ? false : true,
    probes: {
      seed_boot_probe: src.probes && src.probes.seed_boot_probe !== false,
      startup_attestation_verify: src.probes && src.probes.startup_attestation_verify !== false
    }
  };
}

function loadProfile(profilePath = PROFILE_PATH) {
  return normalizeProfile(readJson(profilePath, {}));
}

function enqueueWalletBootstrapProposal(instanceIdRaw: unknown, birthContextRaw: unknown, approvalNoteRaw: unknown) {
  const instanceId = normalizeToken(instanceIdRaw, 160);
  if (!instanceId) {
    return {
      ok: false,
      skipped: true,
      reason: 'wallet_bootstrap_instance_id_missing'
    };
  }
  if (!sovereignBlockchainBridge
    || typeof sovereignBlockchainBridge.loadPolicy !== 'function'
    || typeof sovereignBlockchainBridge.cmdBootstrapProposal !== 'function') {
    return {
      ok: false,
      skipped: true,
      reason: 'wallet_bootstrap_bridge_unavailable'
    };
  }
  try {
    const policyPath = process.env.SOVEREIGN_BLOCKCHAIN_BRIDGE_POLICY_PATH
      ? path.resolve(String(process.env.SOVEREIGN_BLOCKCHAIN_BRIDGE_POLICY_PATH))
      : path.join(ROOT, 'config', 'sovereign_blockchain_bridge_policy.json');
    const policy = sovereignBlockchainBridge.loadPolicy(policyPath);
    if (!policy || policy.enabled !== true) {
      return {
        ok: false,
        skipped: true,
        reason: 'wallet_bootstrap_policy_disabled'
      };
    }
    const result = sovereignBlockchainBridge.cmdBootstrapProposal(policy, {
      'instance-id': instanceId,
      'birth-context': normalizeToken(birthContextRaw, 120) || 'bootstrap',
      'approval-note': cleanText(approvalNoteRaw || 'auto_birth_bootstrap', 320) || 'auto_birth_bootstrap',
      apply: false
    });
    return {
      ok: result && result.ok === true,
      skipped: false,
      proposal_id: result && result.proposal_id ? String(result.proposal_id) : null,
      stage: result && result.stage ? String(result.stage) : null,
      reason_codes: Array.isArray(result && result.reason_codes) ? result.reason_codes.slice(0, 10) : []
    };
  } catch (err) {
    return {
      ok: false,
      skipped: true,
      reason: `wallet_bootstrap_enqueue_failed:${cleanText(err && (err as Error).message || err || 'error', 160)}`
    };
  }
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
  const governanceMissing: string[] = [];
  const governancePaths = Array.isArray(profile.mandatory_governance_paths)
    ? profile.mandatory_governance_paths
    : [];
  for (const relPath of governancePaths) {
    const abs = path.isAbsolute(relPath) ? relPath : path.join(ROOT, relPath);
    if (!fs.existsSync(abs)) governanceMissing.push(relPath);
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
    ok: missing.length === 0
      && governanceMissing.length === 0
      && Object.values(probes).every((row: any) => row && row.ok === true),
    missing,
    missing_governance: governanceMissing,
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

function provisionMinimalCore(profile: AnyObj, targetRoot: string) {
  const root = path.resolve(String(targetRoot || PROVISION_DIR));
  const rows: AnyObj[] = [];
  for (const relPath of (Array.isArray(profile.mandatory_paths) ? profile.mandatory_paths : [])) {
    const copied = copyFileToProvision(String(relPath || ''), root);
    if (copied) rows.push(copied);
  }
  const manifestPath = path.join(root, 'protheus_prime_provision_manifest.json');
  const payload = {
    ok: true,
    type: 'protheus_prime_provision_manifest',
    ts: nowIso(),
    profile_id: profile.profile_id || 'protheus-prime',
    profile_version: profile.version || '1.0',
    file_count: rows.length,
    files: rows
  };
  writeJsonAtomic(manifestPath, payload);
  return {
    ok: true,
    provision_dir: root,
    manifest_path: manifestPath,
    file_count: rows.length,
    files: rows
  };
}

function packagePrimeSeed(profile: AnyObj, evalOut: AnyObj, outDirRaw = PACKAGE_DIR) {
  const outDir = path.resolve(String(outDirRaw || PACKAGE_DIR));
  const stamp = nowIso().replace(/[-:T]/g, '').replace(/\..+$/, 'Z');
  const packageId = `${cleanText(profile.profile_id || 'protheus-prime', 80) || 'protheus-prime'}-${stamp}`.replace(/[^a-zA-Z0-9._-]+/g, '-');
  const target = path.join(outDir, packageId);
  ensureDir(target);

  const files: AnyObj[] = [];
  for (const relPath of (Array.isArray(profile.mandatory_paths) ? profile.mandatory_paths : [])) {
    const safeRel = safeRelPath(String(relPath || ''));
    if (!safeRel) continue;
    const abs = path.join(ROOT, safeRel);
    if (!fs.existsSync(abs)) continue;
    files.push({
      path: safeRel,
      size_bytes: fs.statSync(abs).size,
      sha256: sha256File(abs)
    });
  }
  const packagePayload = {
    ok: evalOut.ok === true,
    type: 'protheus_prime_package',
    ts: nowIso(),
    package_id: packageId,
    package_dir: target,
    profile_id: profile.profile_id,
    profile_version: profile.version,
    strict_bootstrap_ok: evalOut.ok === true,
    missing_paths: Array.isArray(evalOut.missing) ? evalOut.missing : [],
    missing_governance_paths: Array.isArray(evalOut.missing_governance) ? evalOut.missing_governance : [],
    probes: evalOut.probes || {},
    baseline: evalOut.baseline || {},
    file_count: files.length,
    files
  };
  writeJsonAtomic(path.join(target, 'profile.json'), profile);
  writeJsonAtomic(path.join(target, 'bootstrap_eval.json'), evalOut);
  writeJsonAtomic(path.join(target, 'package_manifest.json'), packagePayload);
  writeJsonAtomic(path.join(outDir, 'latest.json'), packagePayload);
  return packagePayload;
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
  const provisionEnabled = args['no-provision'] === true
    ? false
    : (args.provision === false ? false : profile.provision_on_bootstrap !== false);
  const provisionDir = args['provision-dir']
    ? path.resolve(String(args['provision-dir']))
    : PROVISION_DIR;
  const provision = evalOut.ok && provisionEnabled
    ? provisionMinimalCore(profile, provisionDir)
    : { ok: false, skipped: true, reason: evalOut.ok ? 'provision_disabled' : 'bootstrap_not_ok' };
  const walletBootstrapBridge = evalOut.ok
    ? enqueueWalletBootstrapProposal(
      profile.profile_id || 'protheus-prime',
      'bootstrap',
      `auto_birth_bootstrap:${profile.profile_id || 'protheus-prime'}`
    )
    : { ok: false, skipped: true, reason: 'bootstrap_not_ok' };
  const payload = {
    ok: evalOut.ok,
    type: 'protheus_prime_bootstrap',
    ts: nowIso(),
    profile_path: profilePath,
    profile_id: profile.profile_id,
    profile_version: profile.version,
    missing_paths: evalOut.missing,
    missing_governance_paths: evalOut.missing_governance,
    present_paths: evalOut.present,
    probes: evalOut.probes,
    baseline: evalOut.baseline,
    provision,
    wallet_bootstrap_bridge: walletBootstrapBridge
  };
  persistReceipt(payload);
  process.stdout.write(`${JSON.stringify(payload)}\n`);
  if (!evalOut.ok) process.exit(1);
}

function cmdPackage(args: AnyObj) {
  const profilePath = args.profile ? path.resolve(String(args.profile)) : PROFILE_PATH;
  const strict = args.strict === undefined
    ? true
    : String(args.strict) !== '0';
  const outDir = args['out-dir'] ? path.resolve(String(args['out-dir'])) : PACKAGE_DIR;
  const profile = loadProfile(profilePath);
  const evalOut = evaluateBootstrap(profile);
  if (strict && !evalOut.ok) {
    const fail = {
      ok: false,
      type: 'protheus_prime_package',
      ts: nowIso(),
      profile_path: profilePath,
      strict,
      reason: 'bootstrap_conformance_failed',
      missing_paths: evalOut.missing,
      missing_governance_paths: evalOut.missing_governance
    };
    process.stdout.write(`${JSON.stringify(fail)}\n`);
    process.exit(1);
  }
  const pkg = packagePrimeSeed(profile, evalOut, outDir);
  process.stdout.write(`${JSON.stringify(pkg)}\n`);
  if (!pkg.ok && strict) process.exit(1);
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
  if (cmd === 'package') return cmdPackage(args);
  usage();
  process.exit(2);
}

if (require.main === module) {
  main();
}
