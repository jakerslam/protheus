#!/usr/bin/env node
'use strict';
export {};

/**
 * BL-024
 * Event/state schema versioning + validators + controlled migrations.
 *
 * Usage:
 *   node systems/contracts/schema_versioning_gate.js check [--strict=1|0]
 *   node systems/contracts/schema_versioning_gate.js migrate --target=<id>
 *   node systems/contracts/schema_versioning_gate.js status
 */

const fs = require('fs');
const path = require('path');

type AnyObj = Record<string, any>;

type Target = {
  id: string;
  path: string;
  required_schema_id: string;
  min_schema_version: string;
  kind: 'json' | 'jsonl';
};

const ROOT = process.env.SCHEMA_VERSIONING_GATE_ROOT
  ? path.resolve(process.env.SCHEMA_VERSIONING_GATE_ROOT)
  : path.resolve(__dirname, '..', '..');

const DEFAULT_POLICY_PATH = process.env.SCHEMA_VERSIONING_GATE_POLICY_PATH
  ? path.resolve(process.env.SCHEMA_VERSIONING_GATE_POLICY_PATH)
  : path.join(ROOT, 'config', 'schema_versioning_gate_policy.json');

function nowIso() { return new Date().toISOString(); }
function cleanText(v: unknown, maxLen = 360) { return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen); }
function parseArgs(argv: string[]) {
  const out: AnyObj = { _: [] };
  for (let i = 0; i < argv.length; i += 1) {
    const tok = String(argv[i] || '');
    if (!tok.startsWith('--')) { out._.push(tok); continue; }
    const eq = tok.indexOf('=');
    if (eq >= 0) { out[tok.slice(2, eq)] = tok.slice(eq + 1); continue; }
    const key = tok.slice(2);
    const next = argv[i + 1];
    if (next != null && !String(next).startsWith('--')) { out[key] = String(next); i += 1; continue; }
    out[key] = true;
  }
  return out;
}
function toBool(v: unknown, fallback = false) {
  if (v == null) return fallback;
  const raw = String(v).trim().toLowerCase();
  if (['1', 'true', 'yes', 'on'].includes(raw)) return true;
  if (['0', 'false', 'no', 'off'].includes(raw)) return false;
  return fallback;
}
function ensureDir(dirPath: string) { fs.mkdirSync(dirPath, { recursive: true }); }
function readJson(filePath: string, fallback: any = null) {
  try { if (!fs.existsSync(filePath)) return fallback; const parsed = JSON.parse(fs.readFileSync(filePath, 'utf8')); return parsed == null ? fallback : parsed; } catch { return fallback; }
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
function resolvePath(raw: unknown, fallbackRel: string) {
  const txt = cleanText(raw, 520);
  if (!txt) return path.join(ROOT, fallbackRel);
  return path.isAbsolute(txt) ? txt : path.join(ROOT, txt);
}
function rel(filePath: string) { return path.relative(ROOT, filePath).replace(/\\/g, '/'); }
function versionAtLeast(v: string, min: string) {
  const a = String(v || '').split('.').map((x) => Number(x || 0));
  const b = String(min || '').split('.').map((x) => Number(x || 0));
  const n = Math.max(a.length, b.length);
  for (let i = 0; i < n; i += 1) {
    const ai = Number.isFinite(a[i]) ? a[i] : 0;
    const bi = Number.isFinite(b[i]) ? b[i] : 0;
    if (ai > bi) return true;
    if (ai < bi) return false;
  }
  return true;
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    targets: [
      {
        id: 'proposal_admission',
        path: 'config/contracts/proposal_admission.schema.json',
        required_schema_id: 'proposal_admission',
        min_schema_version: '1.0',
        kind: 'json'
      },
      {
        id: 'system_budget',
        path: 'config/contracts/system_budget.schema.json',
        required_schema_id: 'system_budget',
        min_schema_version: '1.0',
        kind: 'json'
      }
    ],
    migrations: {
      target_default_version: '1.0',
      allow_add_missing_fields_only: true
    },
    outputs: {
      latest_path: 'state/contracts/schema_versioning_gate/latest.json',
      history_path: 'state/contracts/schema_versioning_gate/history.jsonl'
    }
  };
}

function normalizeTargets(rows: unknown): Target[] {
  if (!Array.isArray(rows)) return [];
  return rows.map((row: AnyObj, i: number) => ({
    id: cleanText(row && row.id, 120) || `target_${i + 1}`,
    path: cleanText(row && row.path, 520),
    required_schema_id: cleanText(row && row.required_schema_id, 120),
    min_schema_version: cleanText(row && row.min_schema_version, 40) || '1.0',
    kind: cleanText(row && row.kind, 40).toLowerCase() === 'jsonl' ? 'jsonl' : 'json'
  })).filter((row) => row.path && row.required_schema_id);
}

function loadPolicy(policyPath = DEFAULT_POLICY_PATH) {
  const base = defaultPolicy();
  const raw = readJson(policyPath, {});
  const migrations = raw.migrations && typeof raw.migrations === 'object' ? raw.migrations : {};
  const outputs = raw.outputs && typeof raw.outputs === 'object' ? raw.outputs : {};
  const targets = normalizeTargets(raw.targets);

  return {
    version: cleanText(raw.version || base.version, 40) || base.version,
    enabled: raw.enabled !== false,
    targets: targets.length ? targets : normalizeTargets(base.targets),
    migrations: {
      target_default_version: cleanText(migrations.target_default_version || base.migrations.target_default_version, 40) || '1.0',
      allow_add_missing_fields_only: migrations.allow_add_missing_fields_only !== false
    },
    outputs: {
      latest_path: resolvePath(outputs.latest_path, base.outputs.latest_path),
      history_path: resolvePath(outputs.history_path, base.outputs.history_path)
    },
    policy_path: path.resolve(policyPath)
  };
}

function validateTarget(target: Target) {
  const absPath = resolvePath(target.path, target.path);
  if (!fs.existsSync(absPath)) {
    return { id: target.id, ok: false, path: rel(absPath), reason: 'missing_path' };
  }

  if (target.kind === 'json') {
    const payload = readJson(absPath, null);
    if (!payload || typeof payload !== 'object') return { id: target.id, ok: false, path: rel(absPath), reason: 'invalid_json' };
    const schemaId = cleanText(payload.schema_id, 120);
    const schemaVersion = cleanText(payload.schema_version || payload.version, 40);
    const errs: string[] = [];
    if (schemaId !== target.required_schema_id) errs.push('schema_id_mismatch');
    if (!schemaVersion) errs.push('missing_schema_version');
    else if (!versionAtLeast(schemaVersion, target.min_schema_version)) errs.push('schema_version_below_minimum');
    return { id: target.id, ok: errs.length === 0, path: rel(absPath), schema_id: schemaId || null, schema_version: schemaVersion || null, errors: errs };
  }

  const lines = fs.readFileSync(absPath, 'utf8').split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
  const errors: string[] = [];
  let checked = 0;
  for (const line of lines.slice(0, 2000)) {
    let row: AnyObj = null;
    try { row = JSON.parse(line); } catch { errors.push('invalid_jsonl_row'); continue; }
    checked += 1;
    const schemaId = cleanText(row && row.schema_id, 120);
    const schemaVersion = cleanText(row && row.schema_version || row && row.version, 40);
    if (schemaId !== target.required_schema_id) errors.push('schema_id_mismatch');
    if (!schemaVersion) errors.push('missing_schema_version');
    else if (!versionAtLeast(schemaVersion, target.min_schema_version)) errors.push('schema_version_below_minimum');
    if (errors.length > 50) break;
  }
  return {
    id: target.id,
    ok: errors.length === 0,
    path: rel(absPath),
    checked,
    errors: Array.from(new Set(errors)).slice(0, 20)
  };
}

function cmdCheck(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) return { ok: true, strict, result: 'disabled_by_policy', policy_path: rel(policy.policy_path) };

  const validations = policy.targets.map((target: Target) => validateTarget(target));
  const failed = validations.filter((row: AnyObj) => row.ok !== true);

  const out = {
    ok: failed.length === 0,
    ts: nowIso(),
    type: 'schema_versioning_gate',
    strict,
    checked_targets: validations.length,
    failed_targets: failed.length,
    validations,
    policy_path: rel(policy.policy_path)
  };

  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, {
    ts: out.ts,
    type: out.type,
    checked_targets: out.checked_targets,
    failed_targets: out.failed_targets,
    ok: out.ok
  });

  return out;
}

function migrateTarget(target: Target, policy: AnyObj) {
  const absPath = resolvePath(target.path, target.path);
  if (!fs.existsSync(absPath)) return { id: target.id, ok: false, reason: 'missing_path', path: rel(absPath) };
  if (target.kind !== 'json') return { id: target.id, ok: false, reason: 'jsonl_migration_not_supported', path: rel(absPath) };

  const payload = readJson(absPath, null);
  if (!payload || typeof payload !== 'object') return { id: target.id, ok: false, reason: 'invalid_json', path: rel(absPath) };

  const next = { ...payload };
  if (!cleanText(next.schema_id, 120)) next.schema_id = target.required_schema_id;
  if (!cleanText(next.schema_version || next.version, 40)) next.schema_version = policy.migrations.target_default_version;
  if (!next.schema_version && next.version) next.schema_version = cleanText(next.version, 40);

  writeJsonAtomic(absPath, next);
  return {
    id: target.id,
    ok: true,
    migrated_path: rel(absPath),
    schema_id: next.schema_id,
    schema_version: next.schema_version
  };
}

function cmdMigrate(args: AnyObj) {
  const strict = toBool(args.strict, true);
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  if (!policy.enabled) return { ok: true, strict, result: 'disabled_by_policy', policy_path: rel(policy.policy_path) };

  const targetId = cleanText(args.target, 120);
  const targets = targetId ? policy.targets.filter((target: Target) => target.id === targetId) : policy.targets;
  if (!targets.length) return { ok: false, error: 'target_not_found', target: targetId || null };

  const migrations = targets.map((target: Target) => migrateTarget(target, policy));
  const failed = migrations.filter((row: AnyObj) => row.ok !== true);

  const out = {
    ok: failed.length === 0,
    ts: nowIso(),
    type: 'schema_versioning_gate_migration',
    strict,
    target: targetId || 'all',
    migrations,
    policy_path: rel(policy.policy_path)
  };

  writeJsonAtomic(policy.outputs.latest_path, out);
  appendJsonl(policy.outputs.history_path, {
    ts: out.ts,
    type: out.type,
    target: out.target,
    migration_count: migrations.length,
    failed: failed.length,
    ok: out.ok
  });

  return out;
}

function cmdStatus(args: AnyObj) {
  const policyPath = args.policy ? path.resolve(String(args.policy)) : DEFAULT_POLICY_PATH;
  const policy = loadPolicy(policyPath);
  return {
    ok: true,
    ts: nowIso(),
    type: 'schema_versioning_gate_status',
    latest: readJson(policy.outputs.latest_path, null),
    policy_path: rel(policy.policy_path)
  };
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/contracts/schema_versioning_gate.js check [--strict=1|0]');
  console.log('  node systems/contracts/schema_versioning_gate.js migrate --target=<id>');
  console.log('  node systems/contracts/schema_versioning_gate.js status');
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || 'status').toLowerCase();
  if (cmd === 'help' || cmd === '--help' || cmd === '-h') { usage(); return; }

  const payload = cmd === 'check' ? cmdCheck(args)
    : cmd === 'migrate' ? cmdMigrate(args)
      : cmd === 'status' ? cmdStatus(args)
        : { ok: false, error: `unknown_command:${cmd}` };

  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
  if (payload.ok === false && toBool(args.strict, true)) process.exit(1);
  if (payload.ok === false) process.exit(1);
}

if (require.main === module) {
  try { main(); } catch (err) {
    process.stdout.write(`${JSON.stringify({ ok: false, error: cleanText((err as AnyObj)?.message || err || 'schema_versioning_gate_failed', 260) })}\n`);
    process.exit(1);
  }
}

module.exports = { loadPolicy, cmdCheck, cmdMigrate, cmdStatus };
