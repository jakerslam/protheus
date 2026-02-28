#!/usr/bin/env node
'use strict';
export {};

const crypto = require('crypto');
const fs = require('fs');
const path = require('path');
let sovereignBlockchainBridge: AnyObj = null;
try {
  sovereignBlockchainBridge = require('../blockchain/sovereign_blockchain_bridge.js');
} catch {
  sovereignBlockchainBridge = null;
}

type AnyObj = Record<string, any>;

const ROOT = process.env.RESURRECTION_ROOT
  ? path.resolve(process.env.RESURRECTION_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.RESURRECTION_POLICY_PATH
  ? path.resolve(process.env.RESURRECTION_POLICY_PATH)
  : path.join(ROOT, 'config', 'resurrection_protocol_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v: unknown, maxLen = 120) {
  return cleanText(v, maxLen)
    .toLowerCase()
    .replace(/[^a-z0-9_.:/-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function boolFlag(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}

function clampInt(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const i = Math.floor(n);
  if (i < lo) return lo;
  if (i > hi) return hi;
  return i;
}

function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (const tok of argv) {
    if (!String(tok || '').startsWith('--')) {
      out._.push(String(tok || ''));
      continue;
    }
    const idx = tok.indexOf('=');
    if (idx < 0) out[String(tok || '').slice(2)] = true;
    else out[String(tok || '').slice(2, idx)] = String(tok || '').slice(idx + 1);
  }
  return out;
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/continuity/resurrection_protocol.js bundle [--bundle-id=<id>] [--shards=<n>] [--policy=<path>]');
  console.log('  node systems/continuity/resurrection_protocol.js verify --bundle-id=<id> [--policy=<path>] [--strict=1|0]');
  console.log('  node systems/continuity/resurrection_protocol.js restore --bundle-id=<id> --attestation-token=<token> [--target-host=<id>] [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/continuity/resurrection_protocol.js status [--policy=<path>]');
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: AnyObj = {}) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return parsed && typeof parsed === 'object' ? parsed : fallback;
  } catch {
    return fallback;
  }
}

function writeJsonAtomic(filePath: string, value: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function resolvePath(v: unknown) {
  const txt = cleanText(v || '', 400);
  if (!txt) return ROOT;
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}

function shaHex(data: Buffer | string) {
  return crypto.createHash('sha256').update(data).digest('hex');
}

function stableStringify(v: unknown): string {
  if (v == null || typeof v !== 'object') return JSON.stringify(v);
  if (Array.isArray(v)) return `[${v.map((row) => stableStringify(row)).join(',')}]`;
  const obj = v as AnyObj;
  const keys = Object.keys(obj).sort((a, b) => a.localeCompare(b));
  return `{${keys.map((k) => `${JSON.stringify(k)}:${stableStringify(obj[k])}`).join(',')}}`;
}

function defaultPolicy() {
  return {
    schema_id: 'resurrection_protocol_policy',
    schema_version: '1.0',
    enabled: true,
    key_env: 'RESURRECTION_PROTOCOL_KEY',
    key_min_length: 24,
    default_shards: 3,
    max_shards: 16,
    allow_missing_sources: true,
    sources: [
      { path: 'state/continuity/vault/latest.json', required: false },
      { path: 'state/continuity/vault/index.json', required: false },
      { path: 'state/security/soul_token_guard.json', required: false },
      { path: 'config/soul_token_guard_policy.json', required: true },
      { path: 'config/session_continuity_vault_policy.json', required: true },
      { path: 'config/helix_policy.json', required: true },
      { path: 'codex.helix', required: false }
    ],
    state: {
      index_path: 'state/continuity/resurrection/index.json',
      bundles_dir: 'state/continuity/resurrection/bundles',
      recovery_dir: 'state/continuity/resurrection/recovery',
      receipts_path: 'state/continuity/resurrection/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const rawSources = Array.isArray(raw.sources) ? raw.sources : base.sources;
  const sources = rawSources
    .map((row: AnyObj) => ({
      path: cleanText(row && row.path || '', 300),
      required: row && row.required === true
    }))
    .filter((row: AnyObj) => row.path.length > 0);
  const stateRaw = raw.state && typeof raw.state === 'object' ? raw.state : base.state;
  return {
    schema_id: 'resurrection_protocol_policy',
    schema_version: cleanText(raw.schema_version || base.schema_version, 24) || base.schema_version,
    enabled: raw.enabled !== false,
    key_env: cleanText(raw.key_env || base.key_env, 80) || base.key_env,
    key_min_length: clampInt(raw.key_min_length, 8, 4096, base.key_min_length),
    default_shards: clampInt(raw.default_shards, 2, 64, base.default_shards),
    max_shards: clampInt(raw.max_shards, 2, 128, base.max_shards),
    allow_missing_sources: raw.allow_missing_sources !== false,
    sources: sources.length ? sources : base.sources,
    state: {
      index_path: resolvePath(stateRaw.index_path || base.state.index_path),
      bundles_dir: resolvePath(stateRaw.bundles_dir || base.state.bundles_dir),
      recovery_dir: resolvePath(stateRaw.recovery_dir || base.state.recovery_dir),
      receipts_path: resolvePath(stateRaw.receipts_path || base.state.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function resolveKey(policy: AnyObj) {
  const envName = cleanText(policy.key_env || '', 80) || 'RESURRECTION_PROTOCOL_KEY';
  const key = String(process.env[envName] || '');
  if (key.length < Number(policy.key_min_length || 24)) {
    return { ok: false, reason: 'resurrection_key_missing_or_short', env_name: envName };
  }
  return { ok: true, env_name: envName, key };
}

function loadIndex(policy: AnyObj) {
  const src = readJson(policy.state.index_path, { entries: [] });
  return {
    schema_id: 'resurrection_index',
    schema_version: '1.0',
    updated_at: cleanText(src.updated_at || '', 40) || null,
    entries: Array.isArray(src.entries) ? src.entries : []
  };
}

function saveIndex(policy: AnyObj, index: AnyObj) {
  writeJsonAtomic(policy.state.index_path, {
    schema_id: 'resurrection_index',
    schema_version: '1.0',
    updated_at: nowIso(),
    entries: Array.isArray(index.entries) ? index.entries.slice(-500) : []
  });
}

function writeReceipt(policy: AnyObj, row: AnyObj) {
  appendJsonl(policy.state.receipts_path, {
    ts: nowIso(),
    policy_version: policy.schema_version,
    policy_path: rel(policy.policy_path),
    ...row
  });
}

function deriveKey(secret: string, saltB64: string, iterations: number) {
  const salt = Buffer.from(saltB64, 'base64');
  return crypto.pbkdf2Sync(secret, salt, iterations, 32, 'sha256');
}

function encryptPayload(payload: AnyObj, secret: string) {
  const salt = crypto.randomBytes(16).toString('base64');
  const iv = crypto.randomBytes(12).toString('base64');
  const iterations = 120000;
  const key = deriveKey(secret, salt, iterations);
  const cipher = crypto.createCipheriv('aes-256-gcm', key, Buffer.from(iv, 'base64'));
  const plaintext = Buffer.from(stableStringify(payload), 'utf8');
  const ciphertext = Buffer.concat([cipher.update(plaintext), cipher.final()]);
  const tag = cipher.getAuthTag();
  return {
    schema_id: 'resurrection_envelope',
    schema_version: '1.0',
    kdf: 'pbkdf2-sha256',
    iterations,
    salt,
    iv,
    tag: tag.toString('base64'),
    ciphertext: ciphertext.toString('base64'),
    payload_hash: shaHex(plaintext)
  };
}

function decryptPayload(envelope: AnyObj, secret: string) {
  const iterations = clampInt(envelope.iterations, 1000, 5_000_000, 120000);
  const key = deriveKey(secret, String(envelope.salt || ''), iterations);
  const decipher = crypto.createDecipheriv('aes-256-gcm', key, Buffer.from(String(envelope.iv || ''), 'base64'));
  decipher.setAuthTag(Buffer.from(String(envelope.tag || ''), 'base64'));
  const plaintext = Buffer.concat([
    decipher.update(Buffer.from(String(envelope.ciphertext || ''), 'base64')),
    decipher.final()
  ]);
  const payloadHash = shaHex(plaintext);
  if (payloadHash !== String(envelope.payload_hash || '')) throw new Error('resurrection_payload_hash_mismatch');
  return JSON.parse(plaintext.toString('utf8'));
}

function collectSources(policy: AnyObj) {
  const files: AnyObj[] = [];
  const missing: AnyObj[] = [];
  for (const source of policy.sources) {
    const relPath = String(source.path || '').replace(/\\/g, '/').replace(/^\/+/, '');
    if (!relPath || relPath.includes('..')) continue;
    const abs = path.join(ROOT, relPath);
    if (!fs.existsSync(abs) || !fs.statSync(abs).isFile()) {
      missing.push({ path: relPath, required: source.required === true });
      continue;
    }
    const body = fs.readFileSync(abs);
    files.push({
      path: relPath,
      encoding: 'base64',
      data: body.toString('base64'),
      sha256: shaHex(body)
    });
  }
  return { files, missing };
}

function splitShards(buffer: Buffer, shardCount: number) {
  const chunks: Buffer[] = [];
  const size = Math.ceil(buffer.length / shardCount);
  for (let i = 0; i < shardCount; i += 1) {
    const start = i * size;
    const end = Math.min(buffer.length, (i + 1) * size);
    chunks.push(buffer.slice(start, end));
  }
  return chunks;
}

function bundleDir(policy: AnyObj, bundleId: string) {
  return path.join(policy.state.bundles_dir, bundleId);
}

function manifestPath(policy: AnyObj, bundleId: string) {
  return path.join(bundleDir(policy, bundleId), 'manifest.json');
}

function computeRestoreToken(secret: string, bundleId: string, payloadHash: string, targetHost: string) {
  return crypto.createHmac('sha256', secret).update(`${bundleId}|${payloadHash}|${targetHost}`, 'utf8').digest('hex');
}

function enqueueWalletBootstrapProposal(instanceIdRaw: unknown, birthContextRaw: unknown, approvalNoteRaw: unknown) {
  const instanceId = normalizeToken(instanceIdRaw, 160);
  if (!instanceId) return { ok: false, skipped: true, reason: 'wallet_bootstrap_instance_id_missing' };
  if (!sovereignBlockchainBridge
    || typeof sovereignBlockchainBridge.loadPolicy !== 'function'
    || typeof sovereignBlockchainBridge.cmdBootstrapProposal !== 'function') {
    return { ok: false, skipped: true, reason: 'wallet_bootstrap_bridge_unavailable' };
  }
  try {
    const policyPath = process.env.SOVEREIGN_BLOCKCHAIN_BRIDGE_POLICY_PATH
      ? path.resolve(String(process.env.SOVEREIGN_BLOCKCHAIN_BRIDGE_POLICY_PATH))
      : path.join(ROOT, 'config', 'sovereign_blockchain_bridge_policy.json');
    const policy = sovereignBlockchainBridge.loadPolicy(policyPath);
    if (!policy || policy.enabled !== true) return { ok: false, skipped: true, reason: 'wallet_bootstrap_policy_disabled' };
    const out = sovereignBlockchainBridge.cmdBootstrapProposal(policy, {
      'instance-id': instanceId,
      'birth-context': normalizeToken(birthContextRaw, 120) || 'resurrection_restore',
      'approval-note': cleanText(approvalNoteRaw || 'auto_birth_resurrection_restore', 320) || 'auto_birth_resurrection_restore',
      apply: false
    });
    return {
      ok: out && out.ok === true,
      skipped: false,
      proposal_id: out && out.proposal_id ? String(out.proposal_id) : null,
      stage: out && out.stage ? String(out.stage) : null,
      reason_codes: Array.isArray(out && out.reason_codes) ? out.reason_codes.slice(0, 10) : []
    };
  } catch (err) {
    return {
      ok: false,
      skipped: true,
      reason: `wallet_bootstrap_enqueue_failed:${cleanText(err && (err as Error).message || err || 'error', 160)}`
    };
  }
}

function cmdBundle(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  if (policy.enabled !== true) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: 'policy_disabled' }, null, 2)}\n`);
    process.exit(1);
  }
  const keyInfo = resolveKey(policy);
  if (!keyInfo.ok) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: keyInfo.reason, key_env: keyInfo.env_name }, null, 2)}\n`);
    process.exit(1);
  }

  const source = collectSources(policy);
  const missingRequired = source.missing.filter((row) => row.required === true);
  if (missingRequired.length && policy.allow_missing_sources !== true) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: 'required_sources_missing', missing_required: missingRequired }, null, 2)}\n`);
    process.exit(1);
  }

  const bundleId = normalizeToken(args['bundle-id'] || '', 120)
    || `res_${crypto.createHash('sha1').update(`${nowIso()}|${Math.random()}`).digest('hex').slice(0, 12)}`;
  const shardCount = clampInt(args.shards, 2, policy.max_shards, policy.default_shards);

  const payload = {
    schema_id: 'resurrection_payload',
    schema_version: '1.0',
    bundle_id: bundleId,
    created_at: nowIso(),
    root: ROOT,
    files: source.files
  };
  const envelope = encryptPayload(payload, keyInfo.key);
  const envelopeBytes = Buffer.from(JSON.stringify(envelope), 'utf8');
  const shards = splitShards(envelopeBytes, shardCount);

  const outDir = bundleDir(policy, bundleId);
  ensureDir(outDir);
  const shardRows: AnyObj[] = [];
  shards.forEach((chunk, idx) => {
    const fp = path.join(outDir, `shard_${String(idx + 1).padStart(3, '0')}.part`);
    fs.writeFileSync(fp, chunk);
    shardRows.push({
      shard_index: idx + 1,
      file: rel(fp),
      bytes: chunk.length,
      sha256: shaHex(chunk)
    });
  });

  const manifest = {
    schema_id: 'resurrection_manifest',
    schema_version: '1.0',
    bundle_id: bundleId,
    created_at: payload.created_at,
    shard_count: shardRows.length,
    payload_hash: envelope.payload_hash,
    envelope_hash: shaHex(envelopeBytes),
    missing_sources: source.missing,
    shards: shardRows
  };
  writeJsonAtomic(manifestPath(policy, bundleId), manifest);

  const index = loadIndex(policy);
  index.entries.push({
    bundle_id: bundleId,
    created_at: payload.created_at,
    shard_count: shardRows.length,
    payload_hash: envelope.payload_hash,
    files: source.files.length
  });
  saveIndex(policy, index);

  const defaultHost = normalizeToken(args['target-host'] || process.env.RESURRECTION_TARGET_HOST || 'default_host', 120) || 'default_host';
  const restoreToken = computeRestoreToken(keyInfo.key, bundleId, envelope.payload_hash, defaultHost);
  const out = {
    ok: true,
    type: 'resurrection_bundle',
    bundle_id: bundleId,
    manifest_path: rel(manifestPath(policy, bundleId)),
    shard_count: shardRows.length,
    payload_hash: envelope.payload_hash,
    file_count: source.files.length,
    missing_sources: source.missing,
    target_host: defaultHost,
    restore_token: restoreToken
  };
  writeReceipt(policy, out);
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function loadManifest(policy: AnyObj, bundleIdRaw: unknown) {
  const requested = normalizeToken(bundleIdRaw || '', 120);
  const bundleId = requested === 'latest'
    ? normalizeToken((() => {
      const index = loadIndex(policy);
      const latest = Array.isArray(index.entries) && index.entries.length
        ? index.entries[index.entries.length - 1]
        : null;
      return latest && latest.bundle_id ? latest.bundle_id : '';
    })(), 120)
    : requested;
  if (!bundleId) return null;
  const manifest = readJson(manifestPath(policy, bundleId), null);
  if (!manifest || manifest.bundle_id !== bundleId) return null;
  return manifest;
}

function reconstructEnvelope(policy: AnyObj, manifest: AnyObj) {
  const chunks: Buffer[] = [];
  for (const shard of Array.isArray(manifest.shards) ? manifest.shards : []) {
    const fileRel = cleanText(shard.file || '', 320);
    const fp = path.isAbsolute(fileRel) ? fileRel : path.join(ROOT, fileRel);
    if (!fs.existsSync(fp)) throw new Error(`resurrection_shard_missing:${fileRel}`);
    const body = fs.readFileSync(fp);
    if (shaHex(body) !== String(shard.sha256 || '')) throw new Error(`resurrection_shard_hash_mismatch:${fileRel}`);
    chunks.push(body);
  }
  const bytes = Buffer.concat(chunks);
  if (shaHex(bytes) !== String(manifest.envelope_hash || '')) throw new Error('resurrection_envelope_hash_mismatch');
  const envelope = JSON.parse(bytes.toString('utf8'));
  return envelope;
}

function cmdVerify(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const strict = boolFlag(args.strict, false);
  const manifest = loadManifest(policy, args['bundle-id'] || args.bundle_id);
  if (!manifest) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: 'bundle_manifest_not_found' }, null, 2)}\n`);
    if (strict) process.exit(1);
    return;
  }
  const keyInfo = resolveKey(policy);
  if (!keyInfo.ok) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: keyInfo.reason, key_env: keyInfo.env_name }, null, 2)}\n`);
    if (strict) process.exit(1);
    return;
  }

  let payload: AnyObj | null = null;
  try {
    const envelope = reconstructEnvelope(policy, manifest);
    payload = decryptPayload(envelope, keyInfo.key);
  } catch (err) {
    const out = { ok: false, reason: cleanText(err && (err as Error).message || err || 'verify_failed', 200) };
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    if (strict) process.exit(1);
    return;
  }

  const out = {
    ok: true,
    type: 'resurrection_verify',
    bundle_id: manifest.bundle_id,
    shard_count: Number(manifest.shard_count || 0),
    payload_hash: manifest.payload_hash,
    file_count: Array.isArray(payload.files) ? payload.files.length : 0
  };
  writeReceipt(policy, out);
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function cmdRestore(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const manifest = loadManifest(policy, args['bundle-id'] || args.bundle_id);
  if (!manifest) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: 'bundle_manifest_not_found' }, null, 2)}\n`);
    process.exit(1);
  }
  const keyInfo = resolveKey(policy);
  if (!keyInfo.ok) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: keyInfo.reason, key_env: keyInfo.env_name }, null, 2)}\n`);
    process.exit(1);
  }

  const targetHost = normalizeToken(args['target-host'] || process.env.RESURRECTION_TARGET_HOST || 'default_host', 120) || 'default_host';
  const token = cleanText(args['attestation-token'] || args.attestation_token || '', 512);
  const expectedToken = computeRestoreToken(keyInfo.key, manifest.bundle_id, String(manifest.payload_hash || ''), targetHost);
  if (!token || token !== expectedToken) {
    process.stdout.write(`${JSON.stringify({ ok: false, reason: 'restore_attestation_token_mismatch', target_host: targetHost }, null, 2)}\n`);
    process.exit(1);
  }

  const envelope = reconstructEnvelope(policy, manifest);
  const payload = decryptPayload(envelope, keyInfo.key);
  const files = Array.isArray(payload && payload.files) ? payload.files : [];
  const apply = boolFlag(args.apply, false);

  const recoveryRoot = path.join(policy.state.recovery_dir, manifest.bundle_id, nowIso().replace(/[:.]/g, '-'));
  const restored: string[] = [];
  const backups: string[] = [];

  if (apply) {
    for (const row of files) {
      const relPath = String(row && row.path || '').replace(/\\/g, '/').replace(/^\/+/, '');
      if (!relPath || relPath.includes('..')) continue;
      const target = path.join(ROOT, relPath);
      ensureDir(path.dirname(target));
      if (fs.existsSync(target) && fs.statSync(target).isFile()) {
        const backupPath = path.join(recoveryRoot, relPath);
        ensureDir(path.dirname(backupPath));
        fs.copyFileSync(target, backupPath);
        backups.push(rel(backupPath));
      }
      const body = Buffer.from(String(row.data || ''), 'base64');
      fs.writeFileSync(target, body);
      restored.push(rel(target));
    }
  }
  const walletBootstrapBridge = apply
    ? enqueueWalletBootstrapProposal(
      targetHost,
      'resurrection_restore',
      `auto_birth_resurrection_restore:${targetHost}`
    )
    : { ok: false, skipped: true, reason: 'restore_apply_disabled' };

  const out = {
    ok: true,
    type: 'resurrection_restore',
    bundle_id: manifest.bundle_id,
    target_host: targetHost,
    apply,
    file_count: files.length,
    restored,
    backups,
    recovery_root: apply ? rel(recoveryRoot) : null,
    wallet_bootstrap_bridge: walletBootstrapBridge
  };
  writeReceipt(policy, out);
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function cmdStatus(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH);
  const index = loadIndex(policy);
  const latest = index.entries.length ? index.entries[index.entries.length - 1] : null;
  process.stdout.write(`${JSON.stringify({
    ok: true,
    type: 'resurrection_status',
    ts: nowIso(),
    policy_path: rel(policy.policy_path),
    bundles_dir: rel(policy.state.bundles_dir),
    index_path: rel(policy.state.index_path),
    receipts_path: rel(policy.state.receipts_path),
    bundle_count: index.entries.length,
    latest
  }, null, 2)}\n`);
}

function main(argv: string[]) {
  const args = parseArgs(argv);
  const cmd = String(args._[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || args.help) {
    usage();
    process.exit(0);
  }
  if (cmd === 'bundle') return cmdBundle(args);
  if (cmd === 'verify') return cmdVerify(args);
  if (cmd === 'restore') return cmdRestore(args);
  if (cmd === 'status') return cmdStatus(args);
  usage();
  process.exit(2);
}

if (require.main === module) {
  main(process.argv.slice(2));
}

module.exports = {
  DEFAULT_POLICY_PATH,
  loadPolicy,
  computeRestoreToken
};
