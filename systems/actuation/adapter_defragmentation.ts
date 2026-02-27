#!/usr/bin/env node
'use strict';
export {};

const fs = require('fs');
const path = require('path');

type AnyObj = Record<string, any>;

const ROOT = path.resolve(__dirname, '..', '..');
const POLICY_PATH = process.env.ADAPTER_DEFRAGMENTATION_POLICY_PATH
  ? path.resolve(process.env.ADAPTER_DEFRAGMENTATION_POLICY_PATH)
  : path.join(ROOT, 'config', 'adapter_defragmentation_policy.json');

function nowIso() {
  return new Date().toISOString();
}

function dayStr() {
  return nowIso().slice(0, 10);
}

function usage() {
  console.log('Usage:');
  console.log('  node systems/actuation/adapter_defragmentation.js run [YYYY-MM-DD] [--policy=<path>]');
  console.log('  node systems/actuation/adapter_defragmentation.js status [latest|YYYY-MM-DD] [--policy=<path>]');
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

function cleanText(v: unknown, maxLen = 320) {
  return String(v == null ? '' : v).replace(/\s+/g, ' ').trim().slice(0, maxLen);
}

function normalizeToken(v: unknown, maxLen = 120) {
  return cleanText(v, maxLen)
    .toLowerCase()
    .replace(/[^a-z0-9_.:/-]+/g, '_')
    .replace(/_+/g, '_')
    .replace(/^_+|_+$/g, '');
}

function clampInt(v: unknown, lo: number, hi: number, fallback: number) {
  const n = Number(v);
  if (!Number.isFinite(n)) return fallback;
  const i = Math.floor(n);
  if (i < lo) return lo;
  if (i > hi) return hi;
  return i;
}

function ensureDir(dirPath: string) {
  fs.mkdirSync(dirPath, { recursive: true });
}

function readJson(filePath: string, fallback: any) {
  try {
    if (!fs.existsSync(filePath)) return fallback;
    return JSON.parse(fs.readFileSync(filePath, 'utf8'));
  } catch {
    return fallback;
  }
}

function readJsonl(filePath: string) {
  try {
    if (!fs.existsSync(filePath)) return [];
    return fs.readFileSync(filePath, 'utf8')
      .split('\n')
      .filter(Boolean)
      .map((line) => {
        try { return JSON.parse(line); } catch { return null; }
      })
      .filter(Boolean);
  } catch {
    return [];
  }
}

function writeJsonAtomic(filePath: string, payload: AnyObj) {
  ensureDir(path.dirname(filePath));
  const tmp = `${filePath}.tmp-${Date.now()}-${process.pid}`;
  fs.writeFileSync(tmp, `${JSON.stringify(payload, null, 2)}\n`, 'utf8');
  fs.renameSync(tmp, filePath);
}

function appendJsonl(filePath: string, row: AnyObj) {
  ensureDir(path.dirname(filePath));
  fs.appendFileSync(filePath, `${JSON.stringify(row)}\n`, 'utf8');
}

function relPath(filePath: string) {
  return path.relative(ROOT, filePath).replace(/\\/g, '/');
}

function defaultPolicy() {
  return {
    version: '1.0',
    enabled: true,
    adapters_path: 'config/actuation_adapters.json',
    universal_receipts_root: 'state/actuation/universal_execution_primitive/receipts',
    actuation_receipts_root: 'state/actuation/receipts',
    state_root: 'state/actuation/adapter_defragmentation',
    low_usage_threshold: 3,
    profile_ratio_target: 0.8,
    shared_module_hints: [
      'systems/actuation/multi_channel_adapter.js',
      'systems/actuation/universal_execution_primitive.js'
    ],
    exempt_adapters: ['moltbook_publish', 'eyes_create']
  };
}

function loadPolicy(policyPath = POLICY_PATH) {
  const src = readJson(policyPath, {});
  const base = defaultPolicy();
  return {
    version: cleanText(src.version || base.version, 32) || base.version,
    enabled: src.enabled !== false,
    adapters_path: path.resolve(ROOT, cleanText(src.adapters_path || base.adapters_path, 320)),
    universal_receipts_root: path.resolve(ROOT, cleanText(src.universal_receipts_root || base.universal_receipts_root, 320)),
    actuation_receipts_root: path.resolve(ROOT, cleanText(src.actuation_receipts_root || base.actuation_receipts_root, 320)),
    state_root: path.resolve(ROOT, cleanText(src.state_root || base.state_root, 320)),
    low_usage_threshold: clampInt(src.low_usage_threshold, 0, 100000, base.low_usage_threshold),
    profile_ratio_target: Math.max(0, Math.min(1, Number(src.profile_ratio_target != null ? src.profile_ratio_target : base.profile_ratio_target) || base.profile_ratio_target)),
    shared_module_hints: Array.isArray(src.shared_module_hints)
      ? src.shared_module_hints.map((row: unknown) => cleanText(row, 260)).filter(Boolean)
      : base.shared_module_hints.slice(0),
    exempt_adapters: Array.isArray(src.exempt_adapters)
      ? src.exempt_adapters.map((row: unknown) => normalizeToken(row, 80)).filter(Boolean)
      : base.exempt_adapters.slice(0)
  };
}

function countUsage(rows: AnyObj[], keyField: string) {
  const out: Record<string, number> = {};
  for (const row of rows) {
    const key = normalizeToken(row && row[keyField], 80);
    if (!key) continue;
    out[key] = Number(out[key] || 0) + 1;
  }
  return out;
}

function collectSnapshot(policy: AnyObj, date: string) {
  const adapters = readJson(policy.adapters_path, {});
  const adapterMap = adapters && adapters.adapters && typeof adapters.adapters === 'object'
    ? adapters.adapters
    : {};
  const universalRows = readJsonl(path.join(policy.universal_receipts_root, `${date}.jsonl`));
  const actuationRows = readJsonl(path.join(policy.actuation_receipts_root, `${date}.jsonl`));
  const profileUsage = countUsage(universalRows, 'adapter_kind');
  const directUsage = countUsage(actuationRows, 'adapter');

  const adapterRows: AnyObj[] = [];
  const moduleCounts: Record<string, number> = {};
  for (const [rawId, raw] of Object.entries(adapterMap)) {
    const adapterId = normalizeToken(rawId, 80);
    const cfg = raw && typeof raw === 'object' ? raw as AnyObj : {};
    const modulePath = cleanText(cfg.module || '', 260);
    const profileCount = Number(profileUsage[adapterId] || 0);
    const directCount = Number(directUsage[adapterId] || 0);
    const total = profileCount + directCount;
    const profileRatio = total > 0 ? Number((profileCount / total).toFixed(6)) : 0;
    adapterRows.push({
      adapter_id: adapterId,
      module: modulePath,
      profile_runs: profileCount,
      direct_runs: directCount,
      total_runs: total,
      profile_ratio: profileRatio
    });
    const moduleKey = normalizeToken(modulePath, 260);
    if (moduleKey) moduleCounts[moduleKey] = Number(moduleCounts[moduleKey] || 0) + 1;
  }
  adapterRows.sort((a, b) => String(a.adapter_id || '').localeCompare(String(b.adapter_id || '')));

  const sharedHints = new Set((Array.isArray(policy.shared_module_hints) ? policy.shared_module_hints : []).map((row: unknown) => normalizeToken(row, 260)).filter(Boolean));
  const exempt = new Set((Array.isArray(policy.exempt_adapters) ? policy.exempt_adapters : []).map((row: unknown) => normalizeToken(row, 80)).filter(Boolean));
  const candidates = adapterRows
    .filter((row) => !exempt.has(row.adapter_id))
    .filter((row) => !sharedHints.has(normalizeToken(row.module, 260)))
    .filter((row) => Number(row.total_runs || 0) <= Number(policy.low_usage_threshold || 0))
    .map((row) => ({
      adapter_id: row.adapter_id,
      module: row.module,
      total_runs: row.total_runs,
      profile_ratio: row.profile_ratio,
      recommendation: 'migrate_to_universal_execution_primitive'
    }));

  const totalAdapterRuns = adapterRows.reduce((sum, row) => sum + Number(row.total_runs || 0), 0);
  const totalProfileRuns = adapterRows.reduce((sum, row) => sum + Number(row.profile_runs || 0), 0);
  const profileRatio = totalAdapterRuns > 0
    ? Number((totalProfileRuns / totalAdapterRuns).toFixed(6))
    : 0;
  const totalAdapters = adapterRows.length;
  const uniqueModules = Object.keys(moduleCounts).length;
  const consolidationDelta = totalAdapters > 0
    ? Number(((totalAdapters - uniqueModules) / totalAdapters).toFixed(6))
    : 0;

  return {
    ok: true,
    type: 'adapter_defragmentation',
    ts: nowIso(),
    date,
    adapters_path: relPath(policy.adapters_path),
    universal_receipts_path: relPath(path.join(policy.universal_receipts_root, `${date}.jsonl`)),
    actuation_receipts_path: relPath(path.join(policy.actuation_receipts_root, `${date}.jsonl`)),
    profile_ratio_target: Number(policy.profile_ratio_target || 0),
    profile_ratio: profileRatio,
    profile_ratio_target_met: profileRatio >= Number(policy.profile_ratio_target || 0),
    total_adapters: totalAdapters,
    unique_modules: uniqueModules,
    consolidation_delta: consolidationDelta,
    low_usage_threshold: Number(policy.low_usage_threshold || 0),
    candidates,
    adapter_rows: adapterRows
  };
}

function snapshotPaths(policy: AnyObj) {
  return {
    latest: path.join(policy.state_root, 'latest.json'),
    history: path.join(policy.state_root, 'history.jsonl')
  };
}

function cmdRun(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : POLICY_PATH);
  if (!policy.enabled) {
    process.stdout.write(`${JSON.stringify({ ok: false, type: 'adapter_defragmentation', error: 'policy_disabled' })}\n`);
    process.exit(1);
  }
  const date = /^\d{4}-\d{2}-\d{2}$/.test(String(args._[1] || ''))
    ? String(args._[1])
    : dayStr();
  const out = collectSnapshot(policy, date);
  const paths = snapshotPaths(policy);
  writeJsonAtomic(paths.latest, out);
  appendJsonl(paths.history, out);
  process.stdout.write(`${JSON.stringify(out)}\n`);
}

function cmdStatus(args: AnyObj) {
  const policy = loadPolicy(args.policy ? path.resolve(String(args.policy)) : POLICY_PATH);
  const key = cleanText(args._[1] || 'latest', 40);
  if (key !== 'latest') {
    const fp = path.join(policy.state_root, `${key}.json`);
    const out = readJson(fp, null);
    if (!out) {
      process.stdout.write(`${JSON.stringify({ ok: false, type: 'adapter_defragmentation_status', error: 'snapshot_not_found', key })}\n`);
      process.exit(1);
    }
    process.stdout.write(`${JSON.stringify(out)}\n`);
    return;
  }
  const latest = readJson(snapshotPaths(policy).latest, null);
  if (!latest) {
    process.stdout.write(`${JSON.stringify({ ok: false, type: 'adapter_defragmentation_status', error: 'snapshot_not_found', key: 'latest' })}\n`);
    process.exit(1);
  }
  process.stdout.write(`${JSON.stringify(latest)}\n`);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const cmd = String(args._[0] || '').trim().toLowerCase();
  if (!cmd || cmd === 'help' || cmd === '--help' || cmd === '-h' || args.help) {
    usage();
    process.exit(0);
  }
  if (cmd === 'run') return cmdRun(args);
  if (cmd === 'status') return cmdStatus(args);
  usage();
  process.exit(2);
}

if (require.main === module) {
  main();
}
