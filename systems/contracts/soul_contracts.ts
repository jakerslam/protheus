#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-129
 * Soul Contracts Primitive (Immutable User Directive Ledger)
 *
 * User-specific contract payloads are encrypted and stored in memory/.
 * Adaptive contract index lives in adaptive/.
 * Permanent enforcement logic and policy live in systems/ + config/.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampInt,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.SOUL_CONTRACTS_POLICY_PATH
  ? path.resolve(process.env.SOUL_CONTRACTS_POLICY_PATH)
  : path.join(ROOT, 'config', 'soul_contracts_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/contracts/soul_contracts.js create --id=<contract_id> --owner=<owner_id> --title=\"...\" --terms=\"...\" [--risk-tier=2] [--tags=a,b]');
  console.log('  node systems/contracts/soul_contracts.js amend --id=<contract_id> --owner=<owner_id> --terms=\"...\" --tier=3 --approve-a=<sigA> --approve-b=<sigB>');
  console.log('  node systems/contracts/soul_contracts.js evaluate --id=<contract_id> [--owner=<owner_id>] [--action=<name>] [--risk-tier=2]');
  console.log('  node systems/contracts/soul_contracts.js status [--id=<contract_id>] [--owner=<owner_id>]');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    strict_default: true,
    encryption: {
      key_env: 'SOUL_CONTRACTS_KEY',
      allow_dev_key: true,
      dev_key: 'dev_only_replace_in_prod',
      algorithm: 'aes-256-gcm'
    },
    constraints: {
      max_terms_chars: 12000,
      max_title_chars: 240,
      max_contracts_per_owner: 400,
      default_risk_tier: 2,
      max_risk_tier: 4,
      require_dual_signature_tier: 3
    },
    paths: {
      memory_contracts_dir: 'memory/contracts',
      adaptive_index_path: 'adaptive/contracts/index.json',
      latest_path: 'state/contracts/soul_contracts/latest.json',
      history_path: 'state/contracts/soul_contracts/history.jsonl',
      receipts_path: 'state/contracts/soul_contracts/receipts.jsonl'
    }
  };
}

function normalizeTags(raw: unknown) {
  if (Array.isArray(raw)) {
    return raw
      .map((row) => normalizeToken(row, 80))
      .filter(Boolean)
      .slice(0, 24);
  }
  const txt = cleanText(raw || '', 1200);
  if (!txt) return [];
  return txt
    .split(',')
    .map((row) => normalizeToken(row, 80))
    .filter(Boolean)
    .slice(0, 24);
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const encryption = raw.encryption && typeof raw.encryption === 'object' ? raw.encryption : {};
  const constraints = raw.constraints && typeof raw.constraints === 'object' ? raw.constraints : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    strict_default: toBool(raw.strict_default, base.strict_default),
    encryption: {
      key_env: cleanText(encryption.key_env || base.encryption.key_env, 80) || base.encryption.key_env,
      allow_dev_key: toBool(encryption.allow_dev_key, base.encryption.allow_dev_key),
      dev_key: cleanText(encryption.dev_key || base.encryption.dev_key, 240) || base.encryption.dev_key,
      algorithm: cleanText(encryption.algorithm || base.encryption.algorithm, 40) || base.encryption.algorithm
    },
    constraints: {
      max_terms_chars: clampInt(constraints.max_terms_chars, 256, 50000, base.constraints.max_terms_chars),
      max_title_chars: clampInt(constraints.max_title_chars, 16, 2000, base.constraints.max_title_chars),
      max_contracts_per_owner: clampInt(constraints.max_contracts_per_owner, 1, 5000, base.constraints.max_contracts_per_owner),
      default_risk_tier: clampInt(constraints.default_risk_tier, 1, 4, base.constraints.default_risk_tier),
      max_risk_tier: clampInt(constraints.max_risk_tier, 1, 4, base.constraints.max_risk_tier),
      require_dual_signature_tier: clampInt(constraints.require_dual_signature_tier, 1, 4, base.constraints.require_dual_signature_tier)
    },
    paths: {
      memory_contracts_dir: resolvePath(paths.memory_contracts_dir, base.paths.memory_contracts_dir),
      adaptive_index_path: resolvePath(paths.adaptive_index_path, base.paths.adaptive_index_path),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      history_path: resolvePath(paths.history_path, base.paths.history_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function ensureDir(filePath: string) {
  fs.mkdirSync(path.dirname(filePath), { recursive: true });
}

function ownerFile(policy: any, ownerId: string) {
  return path.join(policy.paths.memory_contracts_dir, `${ownerId}.json`);
}

function loadOwnerContracts(policy: any, ownerId: string) {
  const fp = ownerFile(policy, ownerId);
  const row = readJson(fp, { owner_id: ownerId, contracts: [] });
  const contracts = Array.isArray(row && row.contracts) ? row.contracts : [];
  return {
    owner_id: ownerId,
    contracts
  };
}

function saveOwnerContracts(policy: any, payload: any) {
  const fp = ownerFile(policy, payload.owner_id);
  ensureDir(fp);
  writeJsonAtomic(fp, payload);
}

function loadAdaptiveIndex(policy: any) {
  const row = readJson(policy.paths.adaptive_index_path, { contracts: [] });
  return {
    contracts: Array.isArray(row && row.contracts) ? row.contracts : []
  };
}

function saveAdaptiveIndex(policy: any, index: any) {
  ensureDir(policy.paths.adaptive_index_path);
  writeJsonAtomic(policy.paths.adaptive_index_path, index);
}

function resolveKey(policy: any) {
  const keyEnv = cleanText(process.env[policy.encryption.key_env] || '', 4000);
  const source = keyEnv || (policy.encryption.allow_dev_key ? policy.encryption.dev_key : '');
  if (!source) return null;
  return crypto.createHash('sha256').update(String(source), 'utf8').digest();
}

function encryptTerms(policy: any, terms: string) {
  const key = resolveKey(policy);
  if (!key) return { ok: false, error: 'missing_encryption_key' };
  try {
    const iv = crypto.randomBytes(12);
    const cipher = crypto.createCipheriv(policy.encryption.algorithm, key, iv);
    const body = Buffer.concat([cipher.update(String(terms), 'utf8'), cipher.final()]);
    const tag = cipher.getAuthTag();
    return {
      ok: true,
      payload: {
        alg: policy.encryption.algorithm,
        iv_b64: iv.toString('base64'),
        cipher_b64: body.toString('base64'),
        tag_b64: tag.toString('base64'),
        key_id: stableHash(`${policy.encryption.key_env}:${String(key.slice(0, 8).toString('hex'))}`, 20)
      }
    };
  } catch (err) {
    return { ok: false, error: `encrypt_failed:${cleanText(err && err.message, 120) || 'unknown'}` };
  }
}

function rel(absPath: string) {
  return path.relative(ROOT, absPath).replace(/\\/g, '/');
}

function writeReceipts(policy: any, out: any) {
  ensureDir(policy.paths.latest_path);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  appendJsonl(policy.paths.history_path, {
    ts: out.ts,
    action: out.action,
    ok: out.ok,
    contract_id: out.contract_id || null,
    owner_id: out.owner_id || null
  });
}

function upsertAdaptiveIndexRow(index: any, row: any) {
  const rows = Array.isArray(index.contracts) ? index.contracts : [];
  const next = rows.filter((entry: any) => String(entry.contract_id) !== String(row.contract_id));
  next.push(row);
  index.contracts = next.sort((a: any, b: any) => String(a.contract_id).localeCompare(String(b.contract_id)));
}

function createContract(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const contractId = normalizeToken(args.id || args.contract_id, 120);
  const title = cleanText(args.title || '', policy.constraints.max_title_chars);
  const terms = cleanText(args.terms || '', policy.constraints.max_terms_chars);
  const riskTier = clampInt(args['risk-tier'] || args.risk_tier, 1, policy.constraints.max_risk_tier, policy.constraints.default_risk_tier);
  const tags = normalizeTags(args.tags);
  if (!ownerId || !contractId || !title || !terms) {
    return {
      ok: false,
      error: 'missing_required_fields',
      required: ['owner', 'id', 'title', 'terms']
    };
  }

  const owner = loadOwnerContracts(policy, ownerId);
  if (owner.contracts.length >= policy.constraints.max_contracts_per_owner) {
    return {
      ok: false,
      error: 'max_contracts_per_owner_reached',
      owner_id: ownerId,
      max_contracts_per_owner: policy.constraints.max_contracts_per_owner
    };
  }
  if (owner.contracts.some((row: any) => String(row.contract_id) === contractId)) {
    return {
      ok: false,
      error: 'contract_id_exists',
      owner_id: ownerId,
      contract_id: contractId
    };
  }

  const enc = encryptTerms(policy, terms);
  if (!enc.ok) return enc;

  const now = nowIso();
  const row = {
    contract_id: contractId,
    owner_id: ownerId,
    title,
    risk_tier: riskTier,
    tags,
    status: 'active',
    version: 1,
    terms_hash: stableHash(terms, 32),
    terms_encrypted: enc.payload,
    created_at: now,
    updated_at: now,
    amendment_history: []
  };

  owner.contracts.push(row);
  saveOwnerContracts(policy, owner);

  const index = loadAdaptiveIndex(policy);
  upsertAdaptiveIndexRow(index, {
    contract_id: contractId,
    owner_id: ownerId,
    title,
    risk_tier: riskTier,
    status: 'active',
    tags,
    version: 1,
    terms_hash: row.terms_hash,
    updated_at: now
  });
  saveAdaptiveIndex(policy, index);

  return {
    ok: true,
    action: 'create',
    ts: now,
    lane_id: 'V3-RACE-129',
    contract_id: contractId,
    owner_id: ownerId,
    version: 1,
    risk_tier: riskTier,
    terms_hash: row.terms_hash,
    artifacts: {
      owner_file: rel(ownerFile(policy, ownerId)),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function amendContract(policy: any, args: any) {
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const contractId = normalizeToken(args.id || args.contract_id, 120);
  const terms = cleanText(args.terms || '', policy.constraints.max_terms_chars);
  const tier = clampInt(args.tier || args['risk-tier'] || args.risk_tier, 1, policy.constraints.max_risk_tier, policy.constraints.default_risk_tier);
  const approverA = cleanText(args['approve-a'] || args.approve_a || '', 160);
  const approverB = cleanText(args['approve-b'] || args.approve_b || '', 160);

  if (!ownerId || !contractId || !terms) {
    return {
      ok: false,
      error: 'missing_required_fields',
      required: ['owner', 'id', 'terms']
    };
  }
  if (tier < policy.constraints.require_dual_signature_tier) {
    return {
      ok: false,
      error: 'tier_too_low_for_amendment',
      required_tier: policy.constraints.require_dual_signature_tier,
      tier
    };
  }
  if (!approverA || !approverB || approverA === approverB) {
    return {
      ok: false,
      error: 'dual_signature_required',
      required_fields: ['approve-a', 'approve-b'],
      distinct_required: true
    };
  }

  const owner = loadOwnerContracts(policy, ownerId);
  const idx = owner.contracts.findIndex((row: any) => String(row.contract_id) === contractId);
  if (idx < 0) return { ok: false, error: 'contract_not_found', owner_id: ownerId, contract_id: contractId };

  const current = owner.contracts[idx];
  if (String(current.status || '') !== 'active') {
    return { ok: false, error: 'contract_not_active', status: current.status || 'unknown' };
  }

  const enc = encryptTerms(policy, terms);
  if (!enc.ok) return enc;

  const now = nowIso();
  const nextVersion = Math.max(1, Number(current.version || 1)) + 1;
  const amended = {
    ...current,
    version: nextVersion,
    terms_hash: stableHash(terms, 32),
    terms_encrypted: enc.payload,
    updated_at: now,
    amendment_history: [
      ...(Array.isArray(current.amendment_history) ? current.amendment_history : []),
      {
        ts: now,
        version: nextVersion,
        tier,
        approvers: [approverA, approverB]
      }
    ]
  };
  owner.contracts[idx] = amended;
  saveOwnerContracts(policy, owner);

  const index = loadAdaptiveIndex(policy);
  upsertAdaptiveIndexRow(index, {
    contract_id: contractId,
    owner_id: ownerId,
    title: amended.title,
    risk_tier: amended.risk_tier,
    status: amended.status,
    tags: Array.isArray(amended.tags) ? amended.tags : [],
    version: nextVersion,
    terms_hash: amended.terms_hash,
    updated_at: now
  });
  saveAdaptiveIndex(policy, index);

  return {
    ok: true,
    action: 'amend',
    ts: now,
    lane_id: 'V3-RACE-129',
    contract_id: contractId,
    owner_id: ownerId,
    version: nextVersion,
    tier,
    dual_signature: [approverA, approverB],
    terms_hash: amended.terms_hash,
    artifacts: {
      owner_file: rel(ownerFile(policy, ownerId)),
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function evaluateContract(policy: any, args: any) {
  const contractId = normalizeToken(args.id || args.contract_id, 120);
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const action = normalizeToken(args.action || 'unspecified', 120);
  const riskTier = clampInt(args['risk-tier'] || args.risk_tier, 1, policy.constraints.max_risk_tier, policy.constraints.default_risk_tier);
  if (!contractId) return { ok: false, error: 'missing_contract_id' };

  const index = loadAdaptiveIndex(policy);
  const row = index.contracts.find((entry: any) => String(entry.contract_id) === contractId);
  if (!row) {
    return {
      ok: false,
      action: 'evaluate',
      lane_id: 'V3-RACE-129',
      contract_id: contractId,
      owner_id: ownerId || null,
      allow: false,
      reason: 'contract_not_found',
      ts: nowIso()
    };
  }
  if (ownerId && String(row.owner_id) !== ownerId) {
    return {
      ok: false,
      action: 'evaluate',
      lane_id: 'V3-RACE-129',
      contract_id: contractId,
      owner_id: ownerId,
      allow: false,
      reason: 'owner_mismatch',
      ts: nowIso()
    };
  }

  const status = String(row.status || 'unknown');
  const allow = status === 'active';
  return {
    ok: allow,
    action: 'evaluate',
    lane_id: 'V3-RACE-129',
    ts: nowIso(),
    contract_id: contractId,
    owner_id: row.owner_id || ownerId || null,
    requested_action: action,
    requested_risk_tier: riskTier,
    allow,
    reason: allow ? 'contract_active' : 'contract_not_active',
    contract_snapshot: {
      status,
      version: Number(row.version || 0),
      terms_hash: cleanText(row.terms_hash || '', 64)
    }
  };
}

function statusContract(policy: any, args: any) {
  const contractId = normalizeToken(args.id || args.contract_id, 120);
  const ownerId = normalizeToken(args.owner || args.owner_id, 120);
  const index = loadAdaptiveIndex(policy);
  const rows = Array.isArray(index.contracts) ? index.contracts : [];
  const filtered = rows.filter((row: any) => {
    if (contractId && String(row.contract_id) !== contractId) return false;
    if (ownerId && String(row.owner_id) !== ownerId) return false;
    return true;
  });

  return {
    ok: true,
    action: 'status',
    lane_id: 'V3-RACE-129',
    ts: nowIso(),
    contract_count: filtered.length,
    contracts: filtered.slice(0, 200),
    artifacts: {
      adaptive_index_path: rel(policy.paths.adaptive_index_path),
      latest_path: rel(policy.paths.latest_path),
      policy_path: rel(policy.policy_path)
    }
  };
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === 'help' || args.help) {
    usage();
    return emit({ ok: true, type: 'soul_contracts', action: 'help', ts: nowIso() }, 0);
  }

  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) {
    return emit({
      ok: false,
      type: 'soul_contracts',
      action: cmd,
      ts: nowIso(),
      error: 'policy_disabled',
      policy_path: rel(policy.policy_path)
    }, 2);
  }

  let out;
  if (cmd === 'create') out = createContract(policy, args);
  else if (cmd === 'amend') out = amendContract(policy, args);
  else if (cmd === 'evaluate') out = evaluateContract(policy, args);
  else if (cmd === 'status') out = statusContract(policy, args);
  else {
    usage();
    return emit({ ok: false, type: 'soul_contracts', action: cmd, ts: nowIso(), error: 'unknown_command' }, 2);
  }

  const strict = toBool(args.strict, policy.strict_default);
  const payload = {
    ...out,
    type: 'soul_contracts',
    policy_version: policy.version
  };
  writeReceipts(policy, payload);
  return emit(payload, payload.ok || !strict ? 0 : 2);
}

main();
