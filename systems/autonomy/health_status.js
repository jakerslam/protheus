#!/usr/bin/env node
'use strict';

const fs = require('fs');
const path = require('path');
const { spawnSync } = require('child_process');
const { getStopState } = require('../../lib/emergency_stop.js');

const ROOT = path.resolve(__dirname, '..', '..');
const AUTONOMY_CONTROLLER = path.join(ROOT, 'systems', 'autonomy', 'autonomy_controller.js');
const RECEIPT_SUMMARY = path.join(ROOT, 'systems', 'autonomy', 'receipt_summary.js');
const STRATEGY_DOCTOR = path.join(ROOT, 'systems', 'autonomy', 'strategy_doctor.js');
const STRATEGY_READINESS = path.join(ROOT, 'systems', 'autonomy', 'strategy_readiness.js');
const ARCHITECTURE_GUARD = path.join(ROOT, 'systems', 'security', 'architecture_guard.js');
const MODEL_ROUTER = path.join(ROOT, 'systems', 'routing', 'model_router.js');
const ACTUATION_RECEIPTS_DIR = path.join(ROOT, 'state', 'actuation', 'receipts');
const SPINE_HEALTH_PATH = path.join(ROOT, 'state', 'spine', 'router_health.json');
const ROUTING_MODEL_HEALTH_PATH = path.join(ROOT, 'state', 'routing', 'model_health.json');
const AUTONOMY_COOLDOWNS = path.join(ROOT, 'state', 'autonomy', 'cooldowns.json');

function todayStr() { return new Date().toISOString().slice(0, 10); }

function usage() {
  console.log('Usage:');
  console.log('  node systems/autonomy/health_status.js [YYYY-MM-DD]');
  console.log('  node systems/autonomy/health_status.js --help');
}

function runJson(script, args) {
  const r = spawnSync('node', [script, ...args], { cwd: ROOT, encoding: 'utf8' });
  const out = String(r.stdout || '').trim();
  let payload = null;
  if (out) {
    try {
      payload = JSON.parse(out);
    } catch {
      const line = out.split('\n').find(x => x.trim().startsWith('{')) || out;
      try { payload = JSON.parse(line); } catch {}
    }
  }
  return { ok: r.status === 0, code: r.status || 0, payload, stderr: String(r.stderr || '').trim() };
}

function readJson(fp, fallback) {
  try {
    if (!fs.existsSync(fp)) return fallback;
    return JSON.parse(fs.readFileSync(fp, 'utf8'));
  } catch {
    return fallback;
  }
}

function readJsonl(fp) {
  if (!fs.existsSync(fp)) return [];
  return fs.readFileSync(fp, 'utf8').split('\n').filter(Boolean).map((line) => {
    try { return JSON.parse(line); } catch { return null; }
  }).filter(Boolean);
}

function isAttemptedReceipt(rec) {
  if (!rec || typeof rec !== 'object') return false;
  const contract = rec.receipt_contract;
  if (!contract || typeof contract !== 'object') return true;
  return contract.attempted !== false;
}

function actuationReceiptSummary(dateStr) {
  const fp = path.join(ACTUATION_RECEIPTS_DIR, `${dateStr}.jsonl`);
  const rows = readJsonl(fp);
  const attemptedRows = rows.filter(isAttemptedReceipt);
  const out = {
    total: attemptedRows.length,
    skipped_not_attempted: rows.length - attemptedRows.length,
    ok: 0,
    failed: 0,
    verified: 0,
    by_adapter: {}
  };
  for (const r of attemptedRows) {
    const adapter = String(r.adapter || 'unknown');
    out.by_adapter[adapter] = out.by_adapter[adapter] || { total: 0, ok: 0, verified: 0 };
    out.by_adapter[adapter].total += 1;
    if (r.ok === true) {
      out.ok += 1;
      out.by_adapter[adapter].ok += 1;
    } else {
      out.failed += 1;
    }
    const verified = !!(r.receipt_contract && r.receipt_contract.verified === true);
    if (verified) {
      out.verified += 1;
      out.by_adapter[adapter].verified += 1;
    }
  }
  return out;
}

function routingHealthCacheSummary() {
  const snap = readJson(ROUTING_MODEL_HEALTH_PATH, null);
  if (!snap || typeof snap !== 'object') {
    return {
      path: ROUTING_MODEL_HEALTH_PATH,
      schema_version: null,
      active_runtime: null,
      runtimes: [],
      by_runtime_counts: {},
      records_count: 0
    };
  }

  const schemaVersion = Number(snap.schema_version || 0) || null;
  const runtimes = [];
  const byRuntimeCounts = {};

  if (snap.runtimes && typeof snap.runtimes === 'object') {
    for (const [runtime, map] of Object.entries(snap.runtimes)) {
      if (!map || typeof map !== 'object') continue;
      runtimes.push(String(runtime));
      byRuntimeCounts[String(runtime)] = Object.keys(map).length;
    }
  }

  let recordsCount = 0;
  if (snap.records && typeof snap.records === 'object') {
    recordsCount = Object.keys(snap.records).length;
  } else if (!schemaVersion) {
    const legacyKeys = Object.keys(snap).filter((k) => {
      const v = snap[k];
      return !!(v && typeof v === 'object' && typeof v.model === 'string');
    });
    recordsCount = legacyKeys.length;
    if (!runtimes.length && legacyKeys.length) {
      runtimes.push('legacy');
      byRuntimeCounts.legacy = legacyKeys.length;
    }
  }

  return {
    path: ROUTING_MODEL_HEALTH_PATH,
    schema_version: schemaVersion,
    active_runtime: typeof snap.active_runtime === 'string' ? snap.active_runtime : null,
    runtimes,
    by_runtime_counts: byRuntimeCounts,
    records_count: recordsCount
  };
}

function routingDoctorRuntimeSummary(payload) {
  const rows = payload && Array.isArray(payload.diagnostics) ? payload.diagnostics : [];
  if (!rows.length) {
    return {
      total_local_models: 0,
      source_runtime_counts: {},
      stale_local_records: 0,
      local_best_source_runtime: null
    };
  }

  const localRows = rows.filter((r) => r && r.local === true);
  const sourceRuntimeCounts = {};
  let staleLocalRecords = 0;
  for (const row of localRows) {
    const runtime = String((row.local_health && row.local_health.source_runtime) || 'unknown');
    sourceRuntimeCounts[runtime] = Number(sourceRuntimeCounts[runtime] || 0) + 1;
    if (row.local_health && row.local_health.stale === true) staleLocalRecords += 1;
  }

  const localBest = payload && payload.tier1_local_decision && payload.tier1_local_decision.local_best
    ? String(payload.tier1_local_decision.local_best)
    : '';
  const bestRow = localRows.find((r) => String(r.model || '') === localBest);

  return {
    total_local_models: localRows.length,
    source_runtime_counts: sourceRuntimeCounts,
    stale_local_records: staleLocalRecords,
    local_best_source_runtime: bestRow && bestRow.local_health
      ? String(bestRow.local_health.source_runtime || 'unknown')
      : null
  };
}

function main() {
  const arg = process.argv[2] || '';
  if (arg === '--help' || arg === '-h' || arg === 'help') {
    usage();
    process.exit(0);
  }
  const dateStr = /^\d{4}-\d{2}-\d{2}$/.test(arg) ? arg : todayStr();

  const autonomy = runJson(AUTONOMY_CONTROLLER, ['status', dateStr]);
  const receiptSummary = runJson(RECEIPT_SUMMARY, ['run', dateStr, '--days=7']);
  const strategyDoctor = runJson(STRATEGY_DOCTOR, ['run']);
  const strategyReadiness = runJson(STRATEGY_READINESS, ['run', dateStr]);
  const architecture = runJson(ARCHITECTURE_GUARD, ['run']);
  const router = runJson(MODEL_ROUTER, ['doctor', '--risk=low', '--complexity=low', '--intent=autonomy_health', '--task=health']);
  const routingHealth = routingHealthCacheSummary();
  const routingDoctorRuntime = routingDoctorRuntimeSummary(router.payload || null);
  const spineHealth = readJson(SPINE_HEALTH_PATH, { consecutive_full_local_down: 0, last_preflight: null });
  const cooldowns = readJson(AUTONOMY_COOLDOWNS, {});
  const actuation = actuationReceiptSummary(dateStr);

  const out = {
    ok: true,
    date: dateStr,
    operator_tips: {
      model_catalog: [
        'Autonomy can propose/trial model catalog updates automatically.',
        'Applying routing config changes requires elevated approval.',
        'To include new Ollama cloud models from eyes: set AUTONOMY_MODEL_CATALOG_SOURCE=auto (or eye/local).',
        'Optional auto-apply: set AUTONOMY_MODEL_CATALOG_AUTO_APPLY=1 and AUTONOMY_MODEL_CATALOG_AUTO_APPROVAL_NOTE="...".',
        'Run: CLEARANCE=3 node systems/autonomy/model_catalog_loop.js apply --id=<proposal_id> --approval-note="<reason>"'
      ],
      strategy_mode: [
        'Inspect strategy mode/readiness before enabling execution.',
        'Run: node systems/autonomy/strategy_mode.js status',
        'Run: node systems/autonomy/strategy_mode.js recommend YYYY-MM-DD --days=14',
        'Run: node systems/autonomy/strategy_mode_governor.js status YYYY-MM-DD --days=14',
        'Run: node systems/autonomy/strategy_mode_governor.js run YYYY-MM-DD --days=14',
        'Set mode (manual): node systems/autonomy/strategy_mode.js set --mode=execute --approval-note="<reason>" --approver-id="<id1>" --second-approver-id="<id2>" --second-approval-note="<reason2>"'
      ],
      emergency_stop: [
        'Emergency halt for autonomy/routing/actuation when behavior is unsafe.',
        'Engage: node systems/security/emergency_stop.js engage --scope=all --approval-note="<reason>"',
        'Release: node systems/security/emergency_stop.js release --approval-note="<reason>"'
      ]
    },
    routing: {
      spine_local_down_consecutive: Number(spineHealth.consecutive_full_local_down || 0),
      spine_last_preflight: spineHealth.last_preflight || null,
      doctor_ok: router.ok,
      doctor_summary: router.payload && router.payload.tier1_local_decision ? router.payload.tier1_local_decision : null,
      doctor_runtime: routingDoctorRuntime,
      health_cache: routingHealth
    },
    emergency_stop: getStopState(),
    autonomy: autonomy.payload || { ok: false, error: autonomy.stderr || `status_exit_${autonomy.code}` },
    strategy: strategyDoctor.payload || { ok: false, error: strategyDoctor.stderr || `strategy_doctor_exit_${strategyDoctor.code}` },
    strategy_readiness: strategyReadiness.payload || { ok: false, error: strategyReadiness.stderr || `strategy_readiness_exit_${strategyReadiness.code}` },
    autonomy_receipts: receiptSummary.payload || { ok: false, error: receiptSummary.stderr || `receipt_summary_exit_${receiptSummary.code}` },
    architecture_guard: architecture.payload || { ok: false, error: architecture.stderr || `architecture_guard_exit_${architecture.code}` },
    actuation,
    gates: {
      cooldown_count: Object.keys(cooldowns || {}).length
    }
  };

  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
}

main();
