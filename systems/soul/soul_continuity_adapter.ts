#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-020
 *
 * LoRA-backed Soul Continuity Adapter:
 * - export/import/version personality adapter bundles
 * - bind bundles to soul vector hash + identity attestation hash
 * - verify continuity score across migration tests
 * - block promotion when continuity regression exceeds policy thresholds
 */

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const {
  ROOT,
  nowIso,
  parseArgs,
  cleanText,
  normalizeToken,
  toBool,
  clampNumber,
  readJson,
  writeJsonAtomic,
  appendJsonl,
  resolvePath,
  stableHash,
  emit
} = require('../../lib/queued_backlog_runtime');

const POLICY_PATH = process.env.SOUL_CONTINUITY_ADAPTER_POLICY_PATH
  ? path.resolve(process.env.SOUL_CONTINUITY_ADAPTER_POLICY_PATH)
  : path.join(ROOT, 'config', 'soul_continuity_adapter_policy.json');

function usage() {
  console.log('Usage:');
  console.log('  node systems/soul/soul_continuity_adapter.js export --adapter-path=<path> --model=<id> [--format=lora] [--policy=<path>]');
  console.log('  node systems/soul/soul_continuity_adapter.js import --bundle-id=<id> --target-model=<id> [--apply=1|0] [--policy=<path>]');
  console.log('  node systems/soul/soul_continuity_adapter.js verify-migration --bundle-id=<id> --baseline-score=<0..1> --migration-score=<0..1> [--strict=1|0] [--policy=<path>]');
  console.log('  node systems/soul/soul_continuity_adapter.js promote --bundle-id=<id> [--policy=<path>]');
  console.log('  node systems/soul/soul_continuity_adapter.js status [--policy=<path>]');
}

function rel(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    shadow_only: true,
    thresholds: {
      min_continuity_score: 0.9,
      max_regression: 0.05
    },
    scripts: {
      soul_vector_refresh_script: 'systems/symbiosis/soul_vector_substrate.js'
    },
    paths: {
      bundle_dir: 'state/soul/continuity_adapters/bundles',
      state_path: 'state/soul/continuity_adapters/state.json',
      latest_path: 'state/soul/continuity_adapters/latest.json',
      receipts_path: 'state/soul/continuity_adapters/receipts.jsonl',
      soul_vector_latest_path: 'state/symbiosis/soul_vector/latest.json',
      identity_path: 'IDENTITY.md',
      constitution_path: 'AGENT-CONSTITUTION.md'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const thresholds = raw.thresholds && typeof raw.thresholds === 'object' ? raw.thresholds : {};
  const scripts = raw.scripts && typeof raw.scripts === 'object' ? raw.scripts : {};
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: toBool(raw.enabled, true),
    shadow_only: toBool(raw.shadow_only, true),
    thresholds: {
      min_continuity_score: clampNumber(
        thresholds.min_continuity_score,
        0,
        1,
        base.thresholds.min_continuity_score
      ),
      max_regression: clampNumber(
        thresholds.max_regression,
        0,
        1,
        base.thresholds.max_regression
      )
    },
    scripts: {
      soul_vector_refresh_script: resolvePath(
        scripts.soul_vector_refresh_script || base.scripts.soul_vector_refresh_script,
        base.scripts.soul_vector_refresh_script
      )
    },
    paths: {
      bundle_dir: resolvePath(paths.bundle_dir || base.paths.bundle_dir, base.paths.bundle_dir),
      state_path: resolvePath(paths.state_path || base.paths.state_path, base.paths.state_path),
      latest_path: resolvePath(paths.latest_path || base.paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path || base.paths.receipts_path, base.paths.receipts_path),
      soul_vector_latest_path: resolvePath(
        paths.soul_vector_latest_path || base.paths.soul_vector_latest_path,
        base.paths.soul_vector_latest_path
      ),
      identity_path: resolvePath(paths.identity_path || base.paths.identity_path, base.paths.identity_path),
      constitution_path: resolvePath(paths.constitution_path || base.paths.constitution_path, base.paths.constitution_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadState(policy: any) {
  const src = readJson(policy.paths.state_path, null);
  if (!src || typeof src !== 'object') {
    return {
      schema_id: 'soul_continuity_adapter_state',
      schema_version: '1.0',
      updated_at: nowIso(),
      active_bundle_id: null,
      last_bundle_id: null,
      export_count: 0,
      import_count: 0,
      verification_count: 0,
      promotion_count: 0
    };
  }
  return {
    schema_id: 'soul_continuity_adapter_state',
    schema_version: '1.0',
    updated_at: src.updated_at || nowIso(),
    active_bundle_id: src.active_bundle_id || null,
    last_bundle_id: src.last_bundle_id || null,
    export_count: Math.max(0, Number(src.export_count || 0)),
    import_count: Math.max(0, Number(src.import_count || 0)),
    verification_count: Math.max(0, Number(src.verification_count || 0)),
    promotion_count: Math.max(0, Number(src.promotion_count || 0))
  };
}

function saveState(policy: any, state: any) {
  writeJsonAtomic(policy.paths.state_path, {
    schema_id: 'soul_continuity_adapter_state',
    schema_version: '1.0',
    updated_at: nowIso(),
    active_bundle_id: state.active_bundle_id || null,
    last_bundle_id: state.last_bundle_id || null,
    export_count: Math.max(0, Number(state.export_count || 0)),
    import_count: Math.max(0, Number(state.import_count || 0)),
    verification_count: Math.max(0, Number(state.verification_count || 0)),
    promotion_count: Math.max(0, Number(state.promotion_count || 0))
  });
}

function readText(filePath: string) {
  try {
    return String(fs.readFileSync(filePath, 'utf8') || '');
  } catch {
    return '';
  }
}

function runNode(scriptPath: string, args: string[]) {
  if (!fs.existsSync(scriptPath)) return { ok: false, code: 127 };
  const run = spawnSync('node', [scriptPath, ...args], { cwd: ROOT, encoding: 'utf8', timeout: 20000 });
  return {
    ok: Number(run.status || 0) === 0,
    code: Number.isFinite(run.status) ? Number(run.status) : 1
  };
}

function identityAttestation(policy: any) {
  const identity = readText(policy.paths.identity_path);
  const constitution = readText(policy.paths.constitution_path);
  return {
    identity_hash: stableHash(identity, 32),
    constitution_hash: stableHash(constitution, 32),
    attestation_hash: stableHash(`${identity}|${constitution}`, 40)
  };
}

function loadSoulVector(policy: any) {
  let row = readJson(policy.paths.soul_vector_latest_path, null);
  if (!row || typeof row !== 'object') {
    runNode(policy.scripts.soul_vector_refresh_script, ['refresh']);
    row = readJson(policy.paths.soul_vector_latest_path, null);
  }
  if (!row || typeof row !== 'object') {
    return {
      ts: null,
      continuity_fingerprint: null,
      soul_vector_hash: null
    };
  }
  const continuityFingerprint = row.continuity_fingerprint
    || (row.latest && row.latest.continuity_fingerprint)
    || null;
  const soulVectorHash = continuityFingerprint
    ? String(continuityFingerprint)
    : stableHash(JSON.stringify(row), 40);
  return {
    ts: row.ts || row.updated_at || null,
    continuity_fingerprint: continuityFingerprint,
    soul_vector_hash: soulVectorHash
  };
}

function adapterHash(adapterPath: string) {
  try {
    const raw = fs.readFileSync(adapterPath);
    return stableHash(raw.toString('base64'), 40);
  } catch {
    return null;
  }
}

function bundlePath(policy: any, bundleId: string) {
  return path.join(policy.paths.bundle_dir, `${bundleId}.json`);
}

function loadBundle(policy: any, bundleId: string) {
  const id = normalizeToken(bundleId || '', 120);
  if (!id) return null;
  const fp = bundlePath(policy, id);
  const row = readJson(fp, null);
  if (!row || typeof row !== 'object') return null;
  return row;
}

function saveBundle(policy: any, bundle: any) {
  fs.mkdirSync(policy.paths.bundle_dir, { recursive: true });
  writeJsonAtomic(bundlePath(policy, bundle.bundle_id), bundle);
}

function cmdExport(args: any, policy: any) {
  const adapterPath = path.resolve(String(args['adapter-path'] || args.adapter_path || ''));
  if (!adapterPath || !fs.existsSync(adapterPath)) {
    emit({ ok: false, type: 'soul_continuity_export', error: 'adapter_path_required' }, 1);
  }
  const model = normalizeToken(args.model || '', 120);
  if (!model) emit({ ok: false, type: 'soul_continuity_export', error: 'model_required' }, 1);
  const format = normalizeToken(args.format || 'lora', 40) || 'lora';

  const vector = loadSoulVector(policy);
  const attestation = identityAttestation(policy);
  const hash = adapterHash(adapterPath);
  const bundleId = normalizeToken(args['bundle-id'] || `sca_${stableHash(`${model}|${hash}|${Date.now()}`, 12)}`, 120);

  const bundle = {
    schema_id: 'soul_continuity_adapter_bundle',
    schema_version: '1.0',
    bundle_id: bundleId,
    format,
    model,
    adapter_path: adapterPath,
    adapter_path_rel: rel(adapterPath),
    adapter_hash: hash,
    soul_vector_hash: vector.soul_vector_hash,
    soul_vector_ts: vector.ts,
    identity_attestation_hash: attestation.attestation_hash,
    identity_hash: attestation.identity_hash,
    constitution_hash: attestation.constitution_hash,
    version: 1,
    exported_at: nowIso(),
    imported: null,
    latest_verification: null,
    promotion: null
  };
  saveBundle(policy, bundle);

  const state = loadState(policy);
  state.last_bundle_id = bundleId;
  state.export_count += 1;
  saveState(policy, state);

  const out = {
    ok: true,
    type: 'soul_continuity_export',
    ts: nowIso(),
    shadow_only: policy.shadow_only,
    bundle_id: bundleId,
    bundle_path: rel(bundlePath(policy, bundleId)),
    adapter_hash: hash,
    soul_vector_hash: vector.soul_vector_hash,
    identity_attestation_hash: attestation.attestation_hash
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out);
}

function cmdImport(args: any, policy: any) {
  const bundleId = normalizeToken(args['bundle-id'] || '', 120);
  if (!bundleId) emit({ ok: false, type: 'soul_continuity_import', error: 'bundle_id_required' }, 1);
  const targetModel = normalizeToken(args['target-model'] || args.target_model || '', 120) || null;
  const apply = toBool(args.apply, true);

  const bundle = loadBundle(policy, bundleId);
  if (!bundle) emit({ ok: false, type: 'soul_continuity_import', error: 'bundle_not_found', bundle_id: bundleId }, 1);

  const attestation = identityAttestation(policy);
  const attestationMatch = String(bundle.identity_attestation_hash || '') === String(attestation.attestation_hash || '');

  if (apply && attestationMatch) {
    bundle.imported = {
      ts: nowIso(),
      target_model: targetModel || bundle.model,
      attestation_match: true
    };
    saveBundle(policy, bundle);
    const state = loadState(policy);
    state.import_count += 1;
    saveState(policy, state);
  }

  const out = {
    ok: attestationMatch,
    type: 'soul_continuity_import',
    ts: nowIso(),
    bundle_id: bundleId,
    apply,
    target_model: targetModel || bundle.model,
    attestation_match: attestationMatch,
    expected_attestation_hash: bundle.identity_attestation_hash || null,
    actual_attestation_hash: attestation.attestation_hash
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  if (!attestationMatch) emit(out, 1);
  emit(out);
}

function cmdVerifyMigration(args: any, policy: any) {
  const bundleId = normalizeToken(args['bundle-id'] || '', 120);
  if (!bundleId) emit({ ok: false, type: 'soul_continuity_verify_migration', error: 'bundle_id_required' }, 1);
  const bundle = loadBundle(policy, bundleId);
  if (!bundle) emit({ ok: false, type: 'soul_continuity_verify_migration', error: 'bundle_not_found', bundle_id: bundleId }, 1);

  const baseline = clampNumber(args['baseline-score'], 0, 1, NaN);
  const migration = clampNumber(args['migration-score'], 0, 1, NaN);
  if (!Number.isFinite(baseline) || !Number.isFinite(migration)) {
    emit({ ok: false, type: 'soul_continuity_verify_migration', error: 'baseline_score_and_migration_score_required' }, 1);
  }

  const strict = toBool(args.strict, false);
  const regression = Number((baseline - migration).toFixed(6));
  const pass = migration >= Number(policy.thresholds.min_continuity_score || 0)
    && regression <= Number(policy.thresholds.max_regression || 0);

  bundle.latest_verification = {
    ts: nowIso(),
    baseline_score: Number(baseline.toFixed(6)),
    migration_score: Number(migration.toFixed(6)),
    regression,
    min_continuity_score: policy.thresholds.min_continuity_score,
    max_regression: policy.thresholds.max_regression,
    pass
  };
  saveBundle(policy, bundle);

  const state = loadState(policy);
  state.verification_count += 1;
  saveState(policy, state);

  const out = {
    ok: pass,
    type: 'soul_continuity_verify_migration',
    ts: nowIso(),
    bundle_id: bundleId,
    baseline_score: bundle.latest_verification.baseline_score,
    migration_score: bundle.latest_verification.migration_score,
    regression,
    min_continuity_score: policy.thresholds.min_continuity_score,
    max_regression: policy.thresholds.max_regression,
    promotion_blocked: pass !== true
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  if (strict && !pass) emit(out, 1);
  emit(out);
}

function cmdPromote(args: any, policy: any) {
  const bundleId = normalizeToken(args['bundle-id'] || '', 120);
  if (!bundleId) emit({ ok: false, type: 'soul_continuity_promote', error: 'bundle_id_required' }, 1);
  const bundle = loadBundle(policy, bundleId);
  if (!bundle) emit({ ok: false, type: 'soul_continuity_promote', error: 'bundle_not_found', bundle_id: bundleId }, 1);

  const verification = bundle.latest_verification && typeof bundle.latest_verification === 'object'
    ? bundle.latest_verification
    : null;
  if (!verification || verification.pass !== true) {
    emit({
      ok: false,
      type: 'soul_continuity_promote',
      error: 'continuity_regression_threshold_not_met',
      bundle_id: bundleId
    }, 1);
  }

  bundle.promotion = {
    ts: nowIso(),
    promoted: true,
    target_model: bundle.imported && bundle.imported.target_model ? bundle.imported.target_model : bundle.model,
    verification_ref_ts: verification.ts
  };
  saveBundle(policy, bundle);

  const state = loadState(policy);
  state.active_bundle_id = bundleId;
  state.last_bundle_id = bundleId;
  state.promotion_count += 1;
  saveState(policy, state);

  const out = {
    ok: true,
    type: 'soul_continuity_promote',
    ts: nowIso(),
    bundle_id: bundleId,
    active_bundle_id: state.active_bundle_id,
    target_model: bundle.promotion.target_model
  };
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, out);
  emit(out);
}

function cmdStatus(policy: any) {
  const state = loadState(policy);
  const bundles = [];
  if (fs.existsSync(policy.paths.bundle_dir)) {
    const names = fs.readdirSync(policy.paths.bundle_dir)
      .filter((name) => name.endsWith('.json'))
      .sort()
      .slice(-50);
    for (const name of names) {
      const row = readJson(path.join(policy.paths.bundle_dir, name), null);
      if (!row || typeof row !== 'object') continue;
      bundles.push({
        bundle_id: row.bundle_id || null,
        model: row.model || null,
        format: row.format || null,
        exported_at: row.exported_at || null,
        imported: !!row.imported,
        verification_pass: !!(row.latest_verification && row.latest_verification.pass === true),
        promoted: !!(row.promotion && row.promotion.promoted === true)
      });
    }
  }
  emit({
    ok: true,
    type: 'soul_continuity_status',
    ts: nowIso(),
    policy: {
      version: policy.version,
      shadow_only: policy.shadow_only,
      min_continuity_score: policy.thresholds.min_continuity_score,
      max_regression: policy.thresholds.max_regression
    },
    state,
    bundles,
    latest: readJson(policy.paths.latest_path, null),
    paths: {
      bundle_dir: rel(policy.paths.bundle_dir),
      latest_path: rel(policy.paths.latest_path),
      receipts_path: rel(policy.paths.receipts_path)
    }
  });
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = normalizeToken(args._[0] || 'status', 80) || 'status';
  if (cmd === '--help' || cmd === 'help' || cmd === '-h') {
    usage();
    return;
  }
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : POLICY_PATH);
  if (!policy.enabled) emit({ ok: false, error: 'soul_continuity_adapter_disabled' }, 1);

  if (cmd === 'export') return cmdExport(args, policy);
  if (cmd === 'import') return cmdImport(args, policy);
  if (cmd === 'verify-migration') return cmdVerifyMigration(args, policy);
  if (cmd === 'promote') return cmdPromote(args, policy);
  if (cmd === 'status') return cmdStatus(policy);
  usage();
  process.exit(1);
}

main();
