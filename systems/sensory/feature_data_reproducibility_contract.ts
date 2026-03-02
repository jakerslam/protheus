#!/usr/bin/env node
'use strict';
export {};

/**
 * V3-RACE-098
 * Feature/data versioning reproducibility contract.
 */

const fs = require('fs');
const path = require('path');
const crypto = require('crypto');

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.FEATURE_REPRO_POLICY_PATH
  ? path.resolve(process.env.FEATURE_REPRO_POLICY_PATH)
  : path.join(ROOT, 'config', 'feature_data_reproducibility_contract_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function todayStr() {
  return new Date().toISOString().slice(0, 10);
}

function cleanText(v: unknown, maxLen = 260) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function parseArgs(argv: string[]) {
  const out: Record<string, any> = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) {
      out._.push(tok);
      continue;
    }
    const eq = tok.indexOf('=');
    if (eq >= 0) {
      out[tok.slice(2, eq)] = tok.slice(eq + 1);
      continue;
    }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) {
      out[key] = String(next);
      i += 1;
      continue;
    }
    out[key] = true;
  }
  return out;
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: any = null) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8'));
    return parsed == null ? fallback : parsed;
  } catch {
    return fallback;
  }
}

function writeJsonAtomic(filePath: string, value: Record<string, any>) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(value, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: Record<string, any>) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}

function hashText(v: unknown, len = 24) {
  return crypto.createHash('sha256').update(String(v == null ? '' : v), 'utf8').digest('hex').slice(0, len);
}

function normalizeNumber(v: unknown) {
  const n = Number(v);
  return Number.isFinite(n) ? n : 0;
}

function sigmoid(z: number) {
  const x = Math.max(-18, Math.min(18, z));
  return 1 / (1 + Math.exp(-x));
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    decision_threshold: 0.5,
    detector_version_default: 'detector_v1',
    paths: {
      features_dir: 'state/sensory/features',
      output_dir: 'state/sensory/analysis/reproducibility',
      latest_path: 'state/sensory/analysis/reproducibility/latest.json',
      receipts_path: 'state/sensory/analysis/reproducibility/receipts.jsonl'
    }
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const raw = readJson(policyPath, {});
  const base = defaultPolicy();
  const paths = raw.paths && typeof raw.paths === 'object' ? raw.paths : {};
  return {
    version: cleanText(raw.version || base.version, 32) || base.version,
    enabled: raw.enabled !== false,
    decision_threshold: Number.isFinite(Number(raw.decision_threshold)) ? Number(raw.decision_threshold) : base.decision_threshold,
    detector_version_default: cleanText(raw.detector_version_default || base.detector_version_default, 120) || base.detector_version_default,
    paths: {
      features_dir: resolvePath(paths.features_dir, base.paths.features_dir),
      output_dir: resolvePath(paths.output_dir, base.paths.output_dir),
      latest_path: resolvePath(paths.latest_path, base.paths.latest_path),
      receipts_path: resolvePath(paths.receipts_path, base.paths.receipts_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function loadFeaturePack(policy: Record<string, any>, dateStr: string) {
  const fp = path.join(policy.paths.features_dir, `${dateStr}.json`);
  const src = readJson(fp, null);
  const rows = src && Array.isArray(src.rows) ? src.rows : [];
  return {
    file_path: fp,
    rows: rows.filter((row: any) => row && typeof row === 'object')
  };
}

function deriveSchemaHash(rows: Record<string, any>[]) {
  const keys = new Set();
  for (const row of rows || []) {
    for (const key of Object.keys(row || {})) {
      if (key === 'id') continue;
      keys.add(String(key));
    }
  }
  return hashText(Array.from(keys).sort().join('|'), 24);
}

function scoreRows(rows: Record<string, any>[], threshold: number) {
  const decisions = [];
  for (const row of rows || []) {
    const id = cleanText(row && row.id || `row_${decisions.length}`, 120) || `row_${decisions.length}`;
    const featureKeys = Object.keys(row || {}).filter((key) => key !== 'id').sort();
    const z = featureKeys.reduce((sum, key, idx) => sum + (normalizeNumber(row[key]) * (1 / (idx + 1))), 0);
    const probability = sigmoid(z);
    const label = probability >= threshold ? 1 : 0;
    decisions.push({
      id,
      probability: Number(probability.toFixed(6)),
      label
    });
  }
  return decisions;
}

function decisionHash(decisions: Record<string, any>[]) {
  const normalized = decisions
    .map((row) => `${row.id}:${row.probability}:${row.label}`)
    .sort()
    .join('|');
  return hashText(normalized, 24);
}

function run(dateStr: string, policy: Record<string, any>, detectorVersion: string) {
  const pack = loadFeaturePack(policy, dateStr);
  const schemaHash = deriveSchemaHash(pack.rows);
  const featureHash = hashText(JSON.stringify(pack.rows), 24);
  const decisions = scoreRows(pack.rows, Number(policy.decision_threshold || 0.5));
  const decisionsHash = decisionHash(decisions);
  const snapshotId = `snap_${hashText(`${dateStr}|${schemaHash}|${featureHash}|${detectorVersion}`, 20)}`;

  const out = {
    ok: true,
    type: 'feature_data_reproducibility_contract',
    ts: nowIso(),
    date: dateStr,
    snapshot_id: snapshotId,
    source_features_path: pack.file_path,
    detector_version: detectorVersion,
    feature_schema_hash: schemaHash,
    feature_data_hash: featureHash,
    decisions_hash: decisionsHash,
    decision_threshold: Number(policy.decision_threshold || 0.5),
    decision_count: decisions.length,
    decisions
  };

  ensureDir(policy.paths.output_dir);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${dateStr}.json`), out);
  writeJsonAtomic(path.join(policy.paths.output_dir, `${snapshotId}.json`), out);
  writeJsonAtomic(policy.paths.latest_path, out);
  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'feature_repro_snapshot_receipt',
    date: dateStr,
    snapshot_id: snapshotId,
    detector_version: detectorVersion,
    feature_schema_hash: schemaHash,
    feature_data_hash: featureHash,
    decisions_hash: decisionsHash
  });

  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function replay(target: string, policy: Record<string, any>) {
  const probe = /^\d{4}-\d{2}-\d{2}$/.test(String(target || ''))
    ? path.join(policy.paths.output_dir, `${target}.json`)
    : path.join(policy.paths.output_dir, `${target}.json`);
  const snapshot = readJson(probe, null);
  if (!snapshot) {
    const out = { ok: false, error: 'snapshot_not_found', target, path: probe };
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }

  const features = readJson(snapshot.source_features_path, { rows: [] });
  const rows = Array.isArray(features.rows) ? features.rows : [];
  const schemaHash = deriveSchemaHash(rows);
  const featureHash = hashText(JSON.stringify(rows), 24);
  const decisions = scoreRows(rows, Number(snapshot.decision_threshold || policy.decision_threshold || 0.5));
  const decisionsHash = decisionHash(decisions);

  const equivalent = (
    String(schemaHash) === String(snapshot.feature_schema_hash)
    && String(featureHash) === String(snapshot.feature_data_hash)
    && String(decisionsHash) === String(snapshot.decisions_hash)
  );

  const out = {
    ok: equivalent,
    type: 'feature_data_reproducibility_replay',
    ts: nowIso(),
    snapshot_id: snapshot.snapshot_id,
    replay: {
      schema_hash: schemaHash,
      feature_hash: featureHash,
      decisions_hash: decisionsHash,
      equivalent
    }
  };

  appendJsonl(policy.paths.receipts_path, {
    ts: nowIso(),
    type: 'feature_repro_replay_receipt',
    snapshot_id: snapshot.snapshot_id,
    equivalent,
    replay_hash: decisionsHash
  });

  if (!equivalent) {
    process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
    process.exit(2);
  }
  process.stdout.write(`${JSON.stringify(out, null, 2)}\n`);
}

function status(policy: Record<string, any>) {
  const payload = readJson(policy.paths.latest_path, {
    ok: true,
    type: 'feature_data_reproducibility_contract_status',
    snapshot_id: null
  });
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function usageAndExit(code = 0) {
  console.log('Usage:');
  console.log('  node systems/sensory/feature_data_reproducibility_contract.js run [YYYY-MM-DD] [--detector-version=<id>] [--policy=<path>]');
  console.log('  node systems/sensory/feature_data_reproducibility_contract.js replay <snapshot-id|YYYY-MM-DD> [--policy=<path>]');
  console.log('  node systems/sensory/feature_data_reproducibility_contract.js status [--policy=<path>]');
  process.exit(code);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = cleanText(args._[0] || 'status', 40).toLowerCase() || 'status';
  const policy = loadPolicy(args.policy ? String(args.policy) : undefined);
  if (policy.enabled !== true) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: 'policy_disabled' }, null, 2)}\n`);
    process.exit(2);
  }
  if (cmd === 'run') {
    const dateStr = /^\d{4}-\d{2}-\d{2}$/.test(String(args._[1] || '')) ? String(args._[1]) : todayStr();
    const detectorVersion = cleanText(args['detector-version'] || policy.detector_version_default, 120) || policy.detector_version_default;
    return run(dateStr, policy, detectorVersion);
  }
  if (cmd === 'replay') {
    const target = cleanText(args._[1] || '', 160);
    if (!target) return usageAndExit(2);
    return replay(target, policy);
  }
  if (cmd === 'status') return status(policy);
  return usageAndExit(2);
}

module.exports = {
  run,
  replay
};

if (require.main === module) {
  main();
}
